use tauri::{AppHandle, Emitter, Manager};
use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::Disks;
use core_io::{FileBlockReader, BlockReader, PartitionBlockReader};
use disk_layout::{detect_layout, PartitionLayout};
use carver::{carve_drive, CarvedFile, FileCategory};
use fs_ntfs::{NtfsParser, NtfsBootSector, DataInfo};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt, SeekFrom};
use base64::{Engine as _, engine::general_purpose};
use url::Url;
use tauri::http::Response;
use result_store::{ResultStore, DbArtifact};
use rusqlite::params;

#[derive(Serialize)]
pub struct SystemDrive {
    pub id: String,
    pub name: String,
    pub size: String,
    pub used_space: String, // הוספנו את המקום בשימוש!
    pub type_str: String,
    pub is_raw: bool,       // חיווי ל-React אם הכונן דורש פירמוט
}

#[tauri::command]
fn check_admin_rights() -> bool {
    #[cfg(windows)]
    {
        // בווינדוס, רק מנהל יכול לפתוח כונן פיזי לקריאה גולמית
        std::fs::File::open("\\\\.\\PhysicalDrive0").is_ok()
    }
    #[cfg(not(windows))]
    {
        // בלינוקס/מק נוכל לבדוק אם אנחנו משתמש root
        // הערה: נשתמש בבדיקה פשוטה של כתיבה לתיקייה מוגנת או בדיקת UID אם זמין
        // לצורך ה-MVP נבדוק אם אפשר לפתוח את /dev/sda או דומה
        std::fs::File::open("/dev/sda").is_ok() || std::fs::File::open("/dev/disk0").is_ok()
    }
}

#[tauri::command]
async fn get_system_drives() -> Vec<SystemDrive> {
    let mut drives_list = Vec::new();
    let disks = Disks::new_with_refreshed_list();

    for disk in disks.list() {
        let mount_point = disk.mount_point().to_string_lossy().into_owned();
        let os_reported_fs = disk.file_system().to_string_lossy().to_string();
        let total_size_gb = disk.total_space() as f64 / 1_073_741_824.0;

        // נתונים ברירת מחדל ממערכת ההפעלה
        let mut final_fs = format!("מקומי ({})", os_reported_fs);
        let mut is_raw = os_reported_fs.trim().is_empty() || os_reported_fs == "RAW";
        let mut used_gb = (disk.total_space() - disk.available_space()) as f64 / 1_073_741_824.0;

        // בניית הנתיב לקריאה גולמית מתוך הברזל (בווינדוס נוסיף \\.\ לפני אות הכונן)
        let raw_path = if cfg!(target_os = "windows") {
            format!("\\\\.\\{}", mount_point.trim_end_matches('\\'))
        } else {
            mount_point.clone() // בלינוקס/מק זה פשוט הנתיב
        };

        // הנינג'ה נכנסת לפעולה: מנסים לקרוא מהברזל ישירות!
        if let Ok(mut reader) = FileBlockReader::open(&raw_path, 512).await {
            // קוראים את סקטור 0 (ה-VBR)
            if let Ok(boot_sector) = reader.read_exact_at(0, 512).await {
                let real_fs = disk_layout::detect_file_system(&boot_sector);

                // אם Windows חושבת שזה RAW או כונן ריק, אבל אנחנו מצאנו חתימה!
                if is_raw && !matches!(real_fs, disk_layout::FileSystemType::UnknownRaw) {
                    final_fs = format!("RAW (מזוהה {})", real_fs.as_str());
                    is_raw = true; // נסמן ל-UI שזה כונן פגום אבל בר-שחזור

                    // כאן נוכל בהמשך להפעיל את ה-$Bitmap של ה-NTFS כדי לקבל את הנפח בשימוש
                    // לעת עתה, אם מערכת ההפעלה לא יודעת כמה פנוי, נכתוב "לא ידוע":
                    used_gb = 0.0;
                } else {
                    final_fs = real_fs.as_str().to_string();
                }
            }
        }

        let name = disk.name().to_string_lossy().into_owned();
        let display_name = if name.is_empty() { "כונן ללא שם".to_string() } else { name };

        let used_str = if used_gb > 0.0 {
            format!("{:.2} GB בשימוש", used_gb)
        } else if is_raw {
            "דורש סריקה לנפח".to_string()
        } else {
            "0.0 GB בשימוש".to_string()
        };

        drives_list.push(SystemDrive {
            id: raw_path, // אנחנו מעבירים ל-React את הנתיב הגולמי (למשל \\.\E:) שזה מעולה לסריקה!
            name: display_name,
            size: format!("{:.2} GB", total_size_gb),
            used_space: used_str,
            type_str: if disk.is_removable() { format!("USB | {}", final_fs) } else { final_fs },
            is_raw,
        });
    }

    drives_list
}

#[derive(Clone, Serialize)]
struct ScanProgress {
    percentage: f64,
    current_file: String,
    files_found: u32,
    record: Option<fs_ntfs::ParsedRecord>,
}

#[tauri::command]
async fn start_scan(app: AppHandle, path: String, save_path: Option<String>) -> Result<i64, String> {
    println!("מתחיל סריקה על הכונן האמיתי: {}", path);

    // 1. פתיחת הקשר הישיר לכונן הפיזי
    // חובה: התוכנה חייבת לרוץ עם הרשאות מנהל!
    let mut reader = FileBlockReader::open(&path, 512).await
        .map_err(|e| format!("נכשל בפתיחת הכונן. ודא הרשאות מנהל! שגיאה: {:?}", e))?;

    // 2. קריאת מבנה הדיסק וחיפוש המחיצות
    let layout = detect_layout(&mut reader).await
        .map_err(|e| format!("שגיאה בזיהוי מחיצות: {}", e))?;

    // נחפש את מחיצת ה-NTFS הראשונה
    let mut ntfs_start_offset: Option<u64> = None;

    match layout {
        PartitionLayout::MBR(entries) => {
            for entry in entries {
                // קוד 0x07 מייצג NTFS בטבלת MBR
                if entry.partition_type == "0x07" || entry.partition_type == "NTFS" {
                    ntfs_start_offset = Some(entry.start_lba * 512);
                    break;
                }
            }
        },
        PartitionLayout::GPT(entries) => {
            for entry in entries {
                // ה-GUID של מחיצת נתונים ב-Windows (לרוב NTFS)
                if entry.partition_type.contains("Basic Data") || entry.partition_type.contains("NTFS") {
                    ntfs_start_offset = Some(entry.start_lba * 512);
                    break;
                }
            }
        },
        _ => return Err("מבנה מחיצות לא מוכר או לא נתמך.".to_string()),
    }

    let partition_start = ntfs_start_offset.ok_or("לא נמצאה מחיצת NTFS על הכונן!")?;
    println!("מחיצת NTFS נמצאה באופסט: {}", partition_start);

    // 1. הגדרת נתיב שמירה בטוח ל-DB
    let app_data_dir = app.path().app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir()); // גיבוי לתיקייה זמנית

    // נוודא שהתיקייה קיימת
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;

    // ניצור את הקובץ בתוך תיקיית האפליקציה הבטוחה
    let db_path = app_data_dir.join("phoenix_scan.db");
    println!("מסד הנתונים המקומי נשמר בנתיב הבטוח: {:?}", db_path);

    // פתיחת ה-DB עם הנתיב המלא (שים לב שהוספתי mut, נצטרך את זה ל-Transactions)
    let mut db = ResultStore::open(&db_path)
        .map_err(|e| format!("שגיאה בפתיחת מסד הנתונים: {}", e))?;

    // יצירת סשן סריקה חדש
    let session_id = db.create_session(&path).unwrap_or(1);

    // 3. קריאת ה-Boot Sector (VBR) של מחיצת ה-NTFS
    let boot_sector_bytes = reader.read_exact_at(partition_start, 512).await
        .map_err(|e| format!("שגיאה בקריאת ה-Boot Sector: {}", e))?;

    let vbr = NtfsBootSector::parse(&boot_sector_bytes)
        .map_err(|e| format!("שגיאה בפיענוח ה-VBR: {:?}", e))?;

    // 4. חישוב המיקום המוחלט של טבלת ה-MFT על הדיסק הפיזי
    let mft_absolute_offset = partition_start + vbr.mft_offset();
    let cluster_size = (vbr.sectors_per_cluster as u64) * (vbr.bytes_per_sector as u64);
    println!("טבלת MFT נמצאה באופסט מוחלט: {}", mft_absolute_offset);

    // 5. ריצה על רשומות ה-MFT ושידור לממשק
    let mut current_offset = mft_absolute_offset;
    let mut files_found_count = 0;
    let record_size = 1024; // גודל קלאסי של רשומת MFT

    let mut artifact_batch = Vec::new();
    let batch_size_limit = 1000;

    // סורקים למשל את 50,000 הרשומות הראשונות
    for i in 0..50000 {
        // קריאת רשומה אחת
        let record_bytes = match reader.read_exact_at(current_offset, record_size).await {
            Ok(bytes) => bytes,
            Err(_) => break, // הגענו לסוף הדיסק או שיש שגיאת קריאה
        };

        let record = fs_ntfs::parse_mft_record(&record_bytes, i as u64);

        if let Some(filename) = &record.filename {
            // זיהוי MIME עבור הקובץ
            let (_name, _ext, mime, _cat) = if !record.is_directory {
                let sig_json = include_str!("../../../../crates/carver/signatures.json");
                let engine = carver::SignatureEngine::load_from_json(sig_json).unwrap();
                if let Some(DataInfo::Resident(data)) = &record.data_info {
                    engine.identify_smart(data)
                } else {
                    engine.identify_smart(&[0u8; 0])
                }
            } else {
                ("Directory".into(), "".into(), "inode/directory".into(), FileCategory::Unknown)
            };

            artifact_batch.push(DbArtifact {
                session_id,
                mft_id: record.mft_id,
                parent_mft_id: record.parent_mft_id,
                filename: filename.clone(),
                is_dir: record.is_directory,
                is_deleted: record.is_deleted,
                is_resident: record.data_info.as_ref().map_or(false, |d| matches!(d, DataInfo::Resident(_))),
                real_size: match &record.data_info {
                    Some(DataInfo::NonResident { real_size, .. }) => *real_size,
                    Some(DataInfo::Resident(data)) => data.len() as u64,
                    None => 0,
                },
                mime_type: mime,
                disk_offset: current_offset,
            });

            if artifact_batch.len() >= batch_size_limit {
                db.insert_artifacts_batch(&artifact_batch).map_err(|e| e.to_string())?;
                artifact_batch.clear();
            }

            files_found_count += 1;

            if let (Some(dest_dir), Some(data_info)) = (&save_path, &record.data_info) {
                if !record.is_directory {
                    let safe_filename = filename.replace(|c: char| !c.is_alphanumeric() && c != '.', "_");
                    let dest_path = Path::new(dest_dir).join(safe_filename);

                    let mut partition_reader = PartitionBlockReader::new(
                        Box::new(FileBlockReader::open(&path, 512).await.map_err(|e| e.to_string())?),
                        partition_start,
                        reader.total_size() - partition_start,
                    );

                    let _ = fs_ntfs::recover_file(&mut partition_reader, data_info, cluster_size, &dest_path).await;
                }
            }

            let progress = ScanProgress {
                percentage: (i as f64 / 50000.0) * 100.0,
                current_file: filename.clone(),
                files_found: files_found_count,
                record: Some(record),
            };

            let _ = app.emit("scan-progress", progress);
        }

        // מתקדמים לרשומה הבאה
        current_offset += record_size as u64;

        // כדי שהממשק לא יקפא מיותר מדי אירועים (Throttling)
        if i % 100 == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    if !artifact_batch.is_empty() {
        db.insert_artifacts_batch(&artifact_batch).map_err(|e| e.to_string())?;
    }

    let final_progress = ScanProgress {
        percentage: 100.0,
        current_file: "הסריקה הושלמה".to_string(),
        files_found: files_found_count,
        record: None,
    };
    let _ = app.emit("scan-progress", final_progress);

    Ok(session_id)
}

// מבנה דיווח התקדמות עבור השכפול
#[derive(Clone, Serialize)]
struct CloneProgress {
    percentage: f64,
    copied_bytes: u64,
    total_bytes: u64,
}

#[tauri::command]
async fn clone_drive(app: tauri::AppHandle, source_path: String, dest_path: String) -> Result<(), String> {
    println!("מתחיל שכפול: מ-{} אל {}", source_path, dest_path);

    // 1. פתיחת הכונן המקורי לקריאה
    let mut reader = FileBlockReader::open(&source_path, 512).await
        .map_err(|e| format!("נכשל בפתיחת הכונן המקורי: {:?}", e))?;

    let total_size = reader.total_size();

    // 2. פתיחת קובץ היעד לכתיבה
    let mut out_file = tokio::fs::File::create(&dest_path).await
        .map_err(|e| format!("נכשל ביצירת קובץ היעד: {}", e))?;

    // 3. הגדרת גודל מנה (Chunk) - נקרא 10MB בכל פעם לביצועים אופטימליים
    let chunk_size: u64 = 10 * 1024 * 1024;
    let mut current_offset: u64 = 0;
    let mut last_report_time = std::time::Instant::now();

    // 4. לולאת השאיבה
    while current_offset < total_size {
        let to_read = std::cmp::min(chunk_size, total_size - current_offset) as usize;

        // קריאה מהכונן הפיזי
        let data = reader.read_at(current_offset, to_read).await
            .map_err(|e| format!("שגיאת קריאה באופסט {}: {:?}", current_offset, e))?;

        // כתיבה לקובץ האימג'
        out_file.write_all(&data).await
            .map_err(|e| format!("שגיאת כתיבה באופסט {}: {}", current_offset, e))?;

        current_offset += to_read as u64;

        // דיווח התקדמות לממשק (Throttling: מדווחים רק כל חצי שנייה כדי לא להעמיס על ה-UI)
        if last_report_time.elapsed().as_millis() > 500 || current_offset == total_size {
            let progress = CloneProgress {
                percentage: (current_offset as f64 / total_size as f64) * 100.0,
                copied_bytes: current_offset,
                total_bytes: total_size,
            };
            let _ = app.emit("clone-progress", progress);
            last_report_time = std::time::Instant::now();
        }
    }

    println!("השכפול הושלם בהצלחה!");
    Ok(())
}

// מבנים שיעזרו ל-React לתקשר עם Rust
#[derive(Deserialize)]
pub struct ExtractRequest {
    pub source_drive: String, // הכונן הפיזי (למשל \\.\PhysicalDrive1)
    pub output_path: String,  // המיקום לשמירה במחשב
    pub mft_id: u64,          // מזהה הרשומה שנבחרה ב-React
}

#[tauri::command]
async fn extract_file(req: ExtractRequest) -> Result<(), String> {
    println!("מתחיל חילוץ פורנזי לקובץ MFT ID {} אל: {}", req.mft_id, req.output_path);

    // 1. פתיחת הקשר הישיר לכונן
    let mut reader = FileBlockReader::open(&req.source_drive, 512).await
        .map_err(|e| format!("שגיאה בפתיחת הכונן המקורי: {:?}", e))?;

    // 2. איתור מהיר של מחיצת ה-NTFS (בדיוק כמו בסריקה, אבל לוקח אלפית שנייה)
    let layout = disk_layout::detect_layout(&mut reader).await
        .map_err(|e| format!("שגיאה בזיהוי מחיצות: {}", e))?;

    let mut ntfs_start_offset: Option<u64> = None;
    match layout {
        PartitionLayout::MBR(entries) => {
            for entry in entries {
                if entry.partition_type == "0x07" || entry.partition_type == "NTFS" {
                    ntfs_start_offset = Some(entry.start_lba * 512); break;
                }
            }
        },
        PartitionLayout::GPT(entries) => {
            for entry in entries {
                if entry.partition_type.contains("Basic Data") || entry.partition_type.contains("NTFS") {
                    ntfs_start_offset = Some(entry.start_lba * 512); break;
                }
            }
        },
        _ => return Err("מבנה מחיצות לא מוכר.".to_string()),
    }

    let partition_start = ntfs_start_offset.ok_or("לא נמצאה מחיצת NTFS")?;

    // 3. קריאת ה-VBR כדי לדעת איפה ה-MFT ומה גודל הקלאסטר
    let vbr_bytes = reader.read_exact_at(partition_start, 512).await
        .map_err(|e| e.to_string())?;
    let vbr = NtfsBootSector::parse(&vbr_bytes).map_err(|e| format!("{:?}", e))?;

    let bytes_per_cluster = (vbr.bytes_per_sector as u64) * (vbr.sectors_per_cluster as u64);
    let mft_absolute_offset = partition_start + vbr.mft_offset();

    // 4. קפיצה ישירה לרשומה הספציפית! (MFT ID * 1024 bytes)
    let record_offset = mft_absolute_offset + (req.mft_id * 1024);
    let record_bytes = reader.read_exact_at(record_offset, 1024).await
        .map_err(|e| format!("שגיאה בקריאת הרשומה: {}", e))?;

    // מפענחים את הרשומה על המקום כדי לשלוף את הנתונים
    let record_info = fs_ntfs::parse_mft_record(&record_bytes, req.mft_id);
    if record_info.filename.is_none() {
        return Err("נכשל בפיענוח רשומת ה-MFT".to_string());
    }

    // 5. פתיחת קובץ היעד
    let mut out_file = tokio::fs::File::create(&req.output_path).await
        .map_err(|e| format!("שגיאה ביצירת קובץ היעד: {}", e))?;

    // 6. כתיבת הנתונים (Resident או Non-Resident)
    match record_info.data_info {
        Some(DataInfo::Resident(data)) => {
            out_file.write_all(&data).await.map_err(|e| e.to_string())?;
        },
        Some(DataInfo::NonResident { real_size, data_runs, .. }) => {
            let mut bytes_written: u64 = 0;

            for run in data_runs {
                // שים לב: ההיסט של הקלאסטר הוא יחסי לתחילת המחיצה!
                let run_absolute_offset = partition_start + ((run.cluster_offset as u64) * bytes_per_cluster);
                let run_size_bytes = run.cluster_count * bytes_per_cluster;

                let run_data = reader.read_at(run_absolute_offset, run_size_bytes as usize).await
                    .map_err(|e| format!("שגיאה בשאיבת נתונים מהדיסק: {}", e))?;

                // חיתוך עודפים (Slack Space) שלא שייכים לקובץ
                let remaining = real_size - bytes_written;
                let to_write = std::cmp::min(run_data.len() as u64, remaining) as usize;

                out_file.write_all(&run_data[..to_write]).await.map_err(|e| e.to_string())?;
                bytes_written += to_write as u64;

                if bytes_written >= real_size { break; }
            }
        },
        None => return Err("לקובץ זה אין נתונים (ייתכן שהוא ריק או שמדובר בתיקייה)".to_string()),
    }

    Ok(())
}

#[tauri::command]
async fn preview_image(req: ExtractRequest) -> Result<String, String> {
    // 1. פתיחת הקשר הישיר לכונן
    let mut reader = FileBlockReader::open(&req.source_drive, 512).await
        .map_err(|e| format!("שגיאה בפתיחת הכונן המקורי: {:?}", e))?;

    // 2. איתור מהיר של מחיצת ה-NTFS
    let layout = disk_layout::detect_layout(&mut reader).await
        .map_err(|e| format!("שגיאה בזיהוי מחיצות: {}", e))?;

    let mut ntfs_start_offset: Option<u64> = None;
    match layout {
        PartitionLayout::MBR(entries) => {
            for entry in entries {
                if entry.partition_type == "0x07" || entry.partition_type == "NTFS" {
                    ntfs_start_offset = Some(entry.start_lba * 512); break;
                }
            }
        },
        PartitionLayout::GPT(entries) => {
            for entry in entries {
                if entry.partition_type.contains("Basic Data") || entry.partition_type.contains("NTFS") {
                    ntfs_start_offset = Some(entry.start_lba * 512); break;
                }
            }
        },
        _ => return Err("מבנה מחיצות לא מוכר.".to_string()),
    }

    let partition_start = ntfs_start_offset.ok_or("לא נמצאה מחיצת NTFS")?;

    // 3. קריאת ה-VBR
    let vbr_bytes = reader.read_exact_at(partition_start, 512).await
        .map_err(|e| e.to_string())?;
    let vbr = NtfsBootSector::parse(&vbr_bytes).map_err(|e| format!("{:?}", e))?;

    let bytes_per_cluster = (vbr.bytes_per_sector as u64) * (vbr.sectors_per_cluster as u64);
    let mft_absolute_offset = partition_start + vbr.mft_offset();

    // 4. קפיצה לרשומה
    let record_offset = mft_absolute_offset + (req.mft_id * 1024);
    let record_bytes = reader.read_exact_at(record_offset, 1024).await
        .map_err(|e| format!("שגיאה בקריאת הרשומה: {}", e))?;

    let record_info = fs_ntfs::parse_mft_record(&record_bytes, req.mft_id);
    if record_info.filename.is_none() {
        return Err("נכשל בפיענוח רשומת ה-MFT".to_string());
    }

    let mut image_data = Vec::new();

    // 5. איסוף הנתונים לזיכרון
    match record_info.data_info {
        Some(DataInfo::Resident(data)) => {
            image_data = data;
        },
        Some(DataInfo::NonResident { real_size, data_runs, .. }) => {
            // הגבלת גודל לתצוגה מקדימה (10MB)
            if real_size > 10 * 1024 * 1024 {
                return Err("הקובץ גדול מדי לתצוגה מקדימה (מעל 10MB)".to_string());
            }

            let mut bytes_collected: u64 = 0;
            for run in data_runs {
                let run_absolute_offset = partition_start + ((run.cluster_offset as u64) * bytes_per_cluster);
                let run_size_bytes = run.cluster_count * bytes_per_cluster;

                let run_data = reader.read_at(run_absolute_offset, run_size_bytes as usize).await
                    .map_err(|e| format!("שגיאה בשאיבת נתונים: {}", e))?;

                let remaining = real_size - bytes_collected;
                let to_copy = std::cmp::min(run_data.len() as u64, remaining) as usize;
                image_data.extend_from_slice(&run_data[..to_copy]);
                bytes_collected += to_copy as u64;

                if bytes_collected >= real_size { break; }
            }
        },
        None => return Err("אין נתונים לקובץ זה".to_string()),
    }

    // 6. המרה ל-Base64
    let b64 = general_purpose::STANDARD.encode(&image_data);

    // זיהוי סוג התמונה לפי סיומת (בדיקה בסיסית)
    let mime = if let Some(filename) = record_info.filename {
        let filename = filename.to_lowercase();
        if filename.ends_with(".png") { "image/png" }
        else if filename.to_lowercase().ends_with(".gif") { "image/gif" }
        else { "image/jpeg" }
    } else {
        "image/jpeg"
    };

    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
async fn set_partition_active_universal(drive_path: String, partition_index: usize) -> Result<(), String> {
    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&drive_path)
        .await
        .map_err(|e| format!("גישה נדחתה. ודא הרשאות מנהל: {}", e))?;

    // קריאת סקטור 0
    let mut sector_0 = [0u8; 512];
    file.read_exact(&mut sector_0).await.map_err(|e| e.to_string())?;

    // בדיקה: האם זה GPT? (חתימת "EFI PART" נמצאת בסקטור 1)
    let mut sector_1 = [0u8; 512];
    file.seek(SeekFrom::Start(512)).await.map_err(|e| e.to_string())?;
    file.read_exact(&mut sector_1).await.map_err(|e| e.to_string())?;

    if &sector_1[0..8] == b"EFI PART" {
        // --- לוגיקת תיקון GPT ---
        println!("מזהה כונן GPT - מעדכן Attributes");

        // טבלת המחיצות ב-GPT מתחילה לרוב ב-LBA 2 (אופסט 1024)
        // כל רשומה היא 128 בתים. ה-Attributes נמצאים באופסט 48 של הרשומה.
        let partition_entry_offset = 1024 + (partition_index * 128);
        let attribute_offset = partition_entry_offset + 48;

        let mut attr_bytes = [0u8; 8];
        file.seek(SeekFrom::Start(attribute_offset as u64)).await.map_err(|e| e.to_string())?;
        file.read_exact(&mut attr_bytes).await.map_err(|e| e.to_string())?;

        // הדלקת ביט 2 (Legacy BIOS Bootable) - ביט מספר 2 בבייט הראשון (0-indexed)
        attr_bytes[0] |= 1 << 2;

        file.seek(SeekFrom::Start(attribute_offset as u64)).await.map_err(|e| e.to_string())?;
        file.write_all(&attr_bytes).await.map_err(|e| e.to_string())?;

    } else if sector_0[510] == 0x55 && sector_0[511] == 0xAA {
        // --- לוגיקת תיקון MBR ---
        println!("מזהה כונן MBR - מעדכן Boot Indicator");

        for i in 0..4 {
            let offset = 446 + (i * 16);
            sector_0[offset] = if i == partition_index { 0x80 } else { 0x00 };
        }

        file.seek(SeekFrom::Start(0)).await.map_err(|e| e.to_string())?;
        file.write_all(&sector_0).await.map_err(|e| e.to_string())?;
    } else {
        return Err("מבנה כונן לא מזוהה (לא MBR ולא GPT)".to_string());
    }

    file.flush().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct ArtifactDto {
    pub mft_id: u64,
    pub parent_mft_id: u64,
    pub filename: String,
    pub is_dir: bool,
    pub real_size: u64,
    pub mime_type: String,
    pub is_deleted: bool, // NEW
    pub disk_offset: u64, // NEW
}

#[tauri::command]
async fn get_artifacts(app: tauri::AppHandle, session_id: i64) -> Result<Vec<ArtifactDto>, String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir());
    let db_path = app_data_dir.join("phoenix_scan.db");

    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT mft_id, parent_mft_id, filename, is_dir, real_size, mime_type, is_deleted, disk_offset
         FROM artifacts WHERE session_id = ?1 ORDER BY is_dir DESC, filename ASC"
    ).map_err(|e| e.to_string())?;

    let artifact_iter = stmt.query_map(params![session_id], |row| {
        Ok(ArtifactDto {
            mft_id: row.get(0)?,
            parent_mft_id: row.get(1)?,
            filename: row.get(2)?,
            is_dir: row.get(3)?,
            real_size: row.get(4)?,
            mime_type: row.get(5)?,
            is_deleted: row.get(6)?,
            disk_offset: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for artifact in artifact_iter {
        results.push(artifact.map_err(|e| e.to_string())?);
    }

    Ok(results)
}

#[tauri::command]
async fn get_file_preview_data(mft_id: u64, drive: String) -> Result<Vec<u8>, String> {
    // פונקציה לשליפת 4KB ראשונים לתצוגת Hex ולזיהוי מהיר
    let mut reader = FileBlockReader::open(&drive, 512).await
        .map_err(|e| format!("{:?}", e))?;

    // קוראים את הבלוק מה-MFT
    // נניח שהרשומה נמצאת במיקום מוחלט ידוע (ב-MVP ננסה לקרוא ישירות אם זה resident)
    // הערה: במימוש מלא נשתמש ב-NtfsParser למצוא את הנתונים
    let data = reader.read_at(mft_id * 1024, 4096).await
        .map_err(|e| format!("{:?}", e))?;
    Ok(data)
}

#[tauri::command]
fn get_directory_contents(_session_id: i64, _mft_id: u64) -> Result<Vec<ArtifactDto>, String> {
    Ok(Vec::new())
}

#[tauri::command]
async fn deep_scan_carve(app: tauri::AppHandle, path: String) -> Result<(), String> {
    println!("מתחיל סריקה עמוקה (Carving) דינמית על הכונן: {}", path);

    let sig_json = include_str!("../../../../crates/carver/signatures.json");
    let engine = carver::SignatureEngine::load_from_json(sig_json)?;

    let mut reader = core_io::FileBlockReader::open(&path, 512).await
        .map_err(|e| format!("שגיאה בפתיחת הכונן: {:?}", e))?;

    let total_size = reader.total_size();
    let scan_limit = std::cmp::min(total_size, 1024 * 1024 * 1024);

    let found_files = carve_drive(&mut reader, 0, scan_limit, &engine).await?;

    for file in found_files {
        let _ = app.emit("carve-result", file);
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("stream", move |_app, request| {
            let uri = request.uri().to_string();
            let url = Url::parse(&uri).unwrap();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();

            let drive = query.get("drive").cloned().unwrap_or_default();
            let mft_id = query.get("mft_id").and_then(|s| s.parse::<u64>().ok());
            let offset = query.get("offset").and_then(|s| s.parse::<u64>().ok());
            let size = query.get("size").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1 * 1024 * 1024);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let data_res = rt.block_on(async {
                let mut reader = FileBlockReader::open(&drive, 512).await
                    .map_err(|e| format!("{:?}", e))?;

                if let Some(_id) = mft_id {
                    // Placeholder for MFT-based streaming
                    Err("Streaming by MFT ID is not yet implemented".into())
                } else if let Some(off) = offset {
                    reader.read_at(off, size).await
                        .map_err(|e| format!("{:?}", e))
                } else {
                    Err("Missing mft_id or offset".to_string())
                }
            });

            match data_res {
                Ok(data) => {
                    // זיהוי MIME דינמי באמצעות ה-SignatureEngine
                    let sig_json = include_str!("../../../../crates/carver/signatures.json");
                    let engine = carver::SignatureEngine::load_from_json(sig_json).unwrap();
                    let (_name, _ext, mime, _cat) = engine.identify_smart(&data);

                    Response::builder()
                        .header("Content-Type", mime)
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Accept-Ranges", "bytes")
                        .body(data)
                        .unwrap()
                },
                Err(e) => {
                    eprintln!("Stream Error: {}", e);
                    Response::builder()
                        .status(500)
                        .body(Vec::new())
                        .unwrap()
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_scan,
            get_system_drives,
            clone_drive,
            extract_file,
            get_directory_contents,
            check_admin_rights,
            deep_scan_carve,
            preview_image,
            get_file_preview_data,
            get_artifacts,
            set_partition_active_universal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
