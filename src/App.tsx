import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { motion, AnimatePresence } from "motion/react";
import { 
  Database, 
  FileText, 
  Folder, 
  ChevronRight, 
  ChevronDown, 
  Download,
  Search,
  Settings as SettingsIcon,
  HardDrive,
  Usb,
  AlertTriangle,
  RefreshCw,
  ArrowLeft,
  CheckCircle2,
  FileArchive,
  Copy,
  Moon,
  Sun,
  Monitor,
  X,
  Eye,
  Loader2,
  Video,
  Music,
  Image as ImageIcon
} from "lucide-react";
import "./index.css";

interface CarvedFile {
  offset: number;
  file_type: string;
  extension: string;
  estimated_size: number;
  entropy: number;
}

type AppState = "DRIVE_SELECTION" | "SCANNING" | "CLONING" | "RESULTS";
type Theme = "dark" | "light" | "system";

interface DataInfo {
  type: "Resident" | "NonResident";
  value: any;
}

interface ParsedRecord {
  mft_id: number;
  parent_mft_id: number;
  filename: string | null;
  is_directory: boolean;
  data_info: DataInfo | null;
}

interface ScanProgress {
  percentage: number;
  current_file: string;
  files_found: number;
  record: ParsedRecord | null;
}

interface CloneProgress {
  percentage: number;
  copied_bytes: number;
  total_bytes: number;
}

interface SystemDrive {
  id: string;
  name: string;
  size: string;
  used_space: string;
  type_str: string;
  is_raw: boolean;
}

interface MftNode extends ParsedRecord {
  children: number[];
}

// Check if we are running inside Tauri
const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__;

function App() {
  const [appState, setAppState] = useState<AppState>("DRIVE_SELECTION");
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [cloneProgress, setCloneProgress] = useState<CloneProgress | null>(null);
  const [selectedDrive, setSelectedDrive] = useState<string>("");
  const [drives, setDrives] = useState<SystemDrive[]>([]);
  const [isLoadingDrives, setIsLoadingDrives] = useState(true);
  const [fileSystem, setFileSystem] = useState<Record<number, MftNode>>({});
  const [expandedNodes, setExpandedNodes] = useState<Set<number>>(new Set([5]));
  const [selectedNodeId, setSelectedNodeId] = useState<number | null>(null);
  const [theme, setTheme] = useState<Theme>(() => {
    return (localStorage.getItem("theme") as Theme) || "dark";
  });
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isAdmin, setIsAdmin] = useState<boolean>(true);
  const [currentFolderId, setCurrentFolderId] = useState<number>(5); // 5 הוא ה-Root ב-NTFS
  const [folderContents, setFolderContents] = useState<any[]>([]);
  const [navigationHistory, setNavigationHistory] = useState<{id: number, name: string}[]>([{id: 5, name: "Root"}]);
  const [selectedFiles, setSelectedFiles] = useState<Set<number>>(new Set());
  const [carvedFiles, setCarvedFiles] = useState<CarvedFile[]>([]);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);

  const appWindow = isTauri ? getCurrentWindow() : null;

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-theme", theme);
    localStorage.setItem("theme", theme);
  }, [theme]);

  useEffect(() => {
    async function verifyPermissions() {
      if (isTauri) {
        const elevated = await invoke<boolean>("check_admin_rights");
        setIsAdmin(elevated);
      }
    }
    verifyPermissions();
    loadDrives();

    if (!isTauri) {
      console.log("Running in browser mode - Tauri APIs disabled");
      return;
    }

    const unlisten = listen<ScanProgress>("scan-progress", (event) => {
      const payload = event.payload;
      setProgress(payload);

      if (payload.record) {
        const record = payload.record;
        setFileSystem(prev => {
          const newState = { ...prev };
          if (!newState[record.mft_id]) {
            newState[record.mft_id] = { ...record, children: [] };
          } else {
            newState[record.mft_id] = { ...newState[record.mft_id], ...record };
          }
          
          if (record.parent_mft_id !== undefined && record.parent_mft_id !== record.mft_id) {
            if (!newState[record.parent_mft_id]) {
              newState[record.parent_mft_id] = { 
                mft_id: record.parent_mft_id, 
                parent_mft_id: 0, 
                filename: "...", 
                is_directory: true, 
                data_info: null,
                children: [] 
              };
            }
            if (!newState[record.parent_mft_id].children.includes(record.mft_id)) {
              newState[record.parent_mft_id].children.push(record.mft_id);
            }
          }
          return newState;
        });
      }

      if (payload.percentage >= 100) {
        setAppState("RESULTS");
        loadFolder(5, "Root"); // טעינת תיקיית השורש בסיום הסריקה
      }
    });

    const unlistenClone = listen<CloneProgress>("clone-progress", (event) => {
      setCloneProgress(event.payload);
      if (event.payload.percentage >= 100) {
        alert("השכפול הושלם בהצלחה! קובץ האימג' מוכן.");
        setAppState("DRIVE_SELECTION"); // חוזרים למסך הראשי בסיום
      }
    });

    const unlistenCarve = listen<CarvedFile>("carve-result", (event) => {
      setCarvedFiles(prev => [...prev, event.payload]);
    });

    return () => {
      unlisten.then((f) => f());
      unlistenClone.then((f) => f());
      unlistenCarve.then((f) => f());
    };
  }, []);

  async function loadDrives() {
    setIsLoadingDrives(true);
    try {
      if (isTauri) {
        const realDrives = await invoke<SystemDrive[]>("get_system_drives");
        setDrives(realDrives);
      } else {
        // Mock drives for browser preview
        setTimeout(() => {
          setDrives([
            { id: "C:\\", name: "Local Disk", size: "476 GB", type_str: "כונן מקומי (NTFS)" },
            { id: "\\\\.\\PhysicalDrive1", name: "External USB", size: "32 GB", type_str: "USB / נשלף" }
          ]);
          setIsLoadingDrives(false);
        }, 1000);
        return;
      }
    } catch (e) {
      console.error("Failed to load drives:", e);
    } finally {
      if (isTauri) setIsLoadingDrives(false);
    }
  }

  async function startScan(driveId: string) {
    setSelectedDrive(driveId);
    setAppState("SCANNING");
    setFileSystem({});
    setProgress({ percentage: 0, current_file: "Initializing connection...", files_found: 0, record: null });
    
    if (!isTauri) {
      // Simulate scan in browser
      let found = 0;
      const interval = setInterval(() => {
        found += Math.floor(Math.random() * 10);
        const percent = Math.min(100, (found / 100) * 100);
        setProgress({
          percentage: percent,
          current_file: `Simulating: file_${found}.dat`,
          files_found: found,
          record: null
        });
        if (percent >= 100) {
          clearInterval(interval);
          setAppState("RESULTS");
        }
      }, 100);
      return;
    }

    try {
      await invoke("start_scan", { path: driveId });
    } catch (e) {
      console.error(e);
      setAppState("DRIVE_SELECTION");
    }
  }

  // פונקציה לטעינת קובץ אימג' קיים
  async function handleLoadImage() {
    if (!isTauri) {
      alert("טעינת אימג' זמינה רק בגרסת הדסקטופ.");
      return;
    }

    try {
      const selected = await open({
        multiple: false,
        title: "בחר קובץ אימג' לסריקה",
        filters: [{ name: 'Disk Images', extensions: ['img', 'dd', 'raw', 'iso'] }]
      });

      if (selected && typeof selected === 'string') {
        startScan(selected);
      }
    } catch (e) {
      console.error("Failed to open dialog:", e);
    }
  }

  async function handleFixPartition(driveId: string, partitionIndex: number) {
    if (!isTauri) return;
    
    const confirmed = await window.confirm(
      `אזהרה: פעולה זו תשנה את סקטור האתחול של הכונן.\nמומלץ לבצע עותק Image לפני הפעולה.\n\nהאם להמשיך?`
    );

    if (!confirmed) return;

    try {
      await invoke("set_partition_active_universal", { drivePath: driveId, partitionIndex });
      alert("✅ התיקון בוצע בהצלחה! המחיצה הוגדרה כפעילה.");
    } catch (e) {
      alert("❌ שגיאה בתיקון: " + e);
    }
  }

  const renderFixTools = (drive: SystemDrive) => (
    <div style={{
      marginTop: "12px",
      padding: "10px",
      backgroundColor: "rgba(255, 152, 0, 0.05)",
      borderRadius: "8px",
      border: "1px solid rgba(255, 152, 0, 0.2)",
      display: "flex",
      flexDirection: "column",
      gap: "8px"
    }}>
      <div style={{ display: "flex", alignItems: "center", gap: "6px", color: "var(--accent)", fontSize: "12px", fontWeight: 600 }}>
        <SettingsIcon size={14} /> כלי תיקון מהיר
      </div>
      <div style={{ display: "flex", gap: "8px" }}>
        <button 
          className="action-btn"
          style={{ 
            flex: 1, 
            fontSize: "11px", 
            padding: "6px", 
            backgroundColor: "var(--accent)", 
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer"
          }}
          onClick={() => handleFixPartition(drive.id, 0)}
        >
          הגדר Active
        </button>
        <button 
          className="action-btn"
          style={{ 
            flex: 1, 
            fontSize: "11px", 
            padding: "6px", 
            backgroundColor: "transparent", 
            color: "var(--text-dim)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            cursor: "not-allowed",
            opacity: 0.5
          }}
          disabled
          title="בקרוב"
        >
          שחזר VBR
        </button>
      </div>
    </div>
  );

  // פונקציה ליצירת עותק (Clone) של כונן
  async function handleCreateImage(driveId: string, driveName: string) {
    if (!isTauri) {
      alert("יצירת אימג' זמינה רק בגרסת הדסקטופ.");
      return;
    }

    try {
      const savePath = await save({
        title: `שמור עותק של ${driveName}`,
        defaultPath: `${driveName.replace(/[^a-zA-Z0-9]/g, '_')}_backup.img`,
        filters: [{ name: 'Raw Image', extensions: ['img', 'dd'] }]
      });

      if (savePath) {
        setAppState("CLONING");
        setCloneProgress({ percentage: 0, copied_bytes: 0, total_bytes: 100 });
        
        try {
          await invoke("clone_drive", { sourcePath: driveId, destPath: savePath });
        } catch (e) {
          console.error(e);
          alert("שגיאה במהלך יצירת האימג': " + e);
          setAppState("DRIVE_SELECTION");
        }
      }
    } catch (e) {
      console.error("Failed to open save dialog:", e);
    }
  }

  const toggleNode = (id: number) => {
    setExpandedNodes(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // פונקציה שנקראת כשלוחצים על "שחזר" בשורה של קובץ ספציפי
  async function handleRecoverFile(item: any) {
    if (item.is_dir) {
      alert("שחזור תיקיות שלמות יתווסף בקרוב!");
      return; 
    }

    try {
      // 1. פתיחת חלון מערכת ההפעלה לבחירת מיקום שמירה
      const savePath = await save({
        title: `שחזר את: ${item.filename}`,
        defaultPath: item.filename, // מציע אוטומטית את שם הקובץ המקורי
      });

      if (savePath) {
        // 2. קריאה חכמה ורזה ל-Rust
        await invoke("extract_file", {
          req: {
            source_drive: selectedDrive, // הכונן שעליו עשינו את הסריקה
            output_path: savePath,
            mft_id: item.mft_id // הנתון היחיד שהמנוע צריך!
          }
        });
        
        alert(`✅ הקובץ ${item.filename} שוחזר ונשמר בהצלחה אל:\n${savePath}`);
      }
    } catch (e) {
      console.error("שגיאה בחילוץ הקובץ:", e);
      alert("שגיאה בחילוץ הקובץ: " + e);
    }
  }

  // פונקציה לטעינת תוכן תיקייה מה-SQLite
  async function loadFolder(mftId: number, folderName: string) {
    if (!isTauri) {
      // בסימולציה בדפדפן - נציג נתונים פיקטיביים
      setFolderContents([
        { mft_id: mftId + 1, filename: "Documents", is_dir: true, real_size: 0 },
        { mft_id: mftId + 2, filename: "Photos", is_dir: true, real_size: 0 },
        { mft_id: mftId + 3, filename: "report.pdf", is_dir: false, real_size: 1024 * 500 },
      ]);
      return;
    }

    try {
      // נניח ש-sessionId הוא 1 כרגע (בפרויקט מלא תשמור את ה-ID מהסריקה)
      const contents = await invoke<any[]>("get_directory_contents", { sessionId: 1, parentMftId: mftId });
      setFolderContents(contents);
      setCurrentFolderId(mftId);
      
      // הוספה להיסטוריית הניווט (Breadcrumbs)
      setNavigationHistory(prev => {
        const existingIndex = prev.findIndex(h => h.id === mftId);
        if (existingIndex !== -1) {
          return prev.slice(0, existingIndex + 1);
        } else {
          return [...prev, {id: mftId, name: folderName}];
        }
      });
    } catch (e) {
      console.error("שגיאה בטעינת תיקייה:", e);
    }
  }

  async function handleRecoverSelected() {
    if (selectedFiles.size === 0) return;

    if (!isTauri) {
      alert("שחזור מרוכז זמין רק בגרסת הדסקטופ.");
      return;
    }

    try {
      const outputDir = await open({
        directory: true,
        multiple: false,
        title: "בחר תיקייה לשמירת הקבצים המשוחזרים"
      });

      if (outputDir && typeof outputDir === 'string') {
        const filesToRecover = folderContents.filter(f => selectedFiles.has(f.mft_id));
        console.log(`מתחיל שחזור של ${filesToRecover.length} קבצים אל ${outputDir}`);

        let successCount = 0;
        for (const file of filesToRecover) {
          try {
            await invoke("extract_file", {
              req: {
                source_drive: selectedDrive,
                output_path: `${outputDir}\\${file.filename}`, 
                mft_id: file.mft_id
              }
            });
            successCount++;
          } catch (e) {
            console.error(`שגיאה בשחזור ${file.filename}:`, e);
          }
        }
        
        alert(`✅ ${successCount} מתוך ${filesToRecover.length} קבצים שוחזרו בהצלחה!`);
        setSelectedFiles(new Set()); 
      }
    } catch (e) {
      console.error("שגיאה בשחזור מרוכז:", e);
    }
  }

  async function handlePreview(item: any) {
    if (item.is_dir) return;
    
    const isImage = item.filename.toLowerCase().match(/\.(jpg|jpeg|png|gif|bmp|webp)$/i);
    const isVideo = item.filename.toLowerCase().match(/\.(mp4|mkv|avi|mov|wmv)$/i);
    const isAudio = item.filename.toLowerCase().match(/\.(mp3|wav|ogg|flac|m4a)$/i);

    if (!isImage && !isVideo && !isAudio) {
      alert("תצוגה מקדימה זמינה רק לתמונות, וידאו ואודיו כרגע.");
      return;
    }

    if (!isTauri) {
      alert("תצוגה מקדימה זמינה רק בגרסת הדסקטופ.");
      return;
    }

    setIsLoadingPreview(true);
    setPreviewUrl(null);
    
    // שימוש בפרוטוקול הסטרימינג החדש
    const streamUrl = `phoenix-stream://file?drive=${encodeURIComponent(selectedDrive)}&mft_id=${item.mft_id}&filename=${encodeURIComponent(item.filename)}`;
    
    setPreviewUrl(streamUrl);
    setIsLoadingPreview(false);
  }

  const renderMediaPreview = () => {
    if (!previewUrl) return null;

    const isVideo = previewUrl.toLowerCase().match(/\.(mp4|mkv|avi|mov|wmv)/i);
    const isAudio = previewUrl.toLowerCase().match(/\.(mp3|wav|ogg|flac|m4a)/i);

    if (isVideo) {
      return (
        <div className="media-preview-container">
          <video controls className="preview-media" autoPlay>
            <source src={previewUrl} />
            הדפדפן שלך לא תומך בנגן הוידאו.
          </video>
          <div className="media-info">🎥 הזרמה ישירה מהסקטורים...</div>
        </div>
      );
    }

    if (isAudio) {
      return (
        <div className="media-preview-container audio-preview">
          <div className="audio-icon-large">🎵</div>
          <audio controls className="preview-media" autoPlay>
            <source src={previewUrl} />
          </audio>
          <div className="media-info">🎧 הזרמת אודיו בזמן אמת</div>
        </div>
      );
    }

    return (
      <img 
        src={previewUrl} 
        alt="Preview" 
        className="preview-image"
        referrerPolicy="no-referrer"
      />
    );
  };

  const renderTreeNode = (id: number, depth = 0) => {
    const node = fileSystem[id];
    if (!node || (!node.filename && id !== 5)) return null;

    const isExpanded = expandedNodes.has(id);
    const hasChildren = node.children.length > 0;
    const isSelected = selectedNodeId === id;

    return (
      <div key={id} className="tree-node-wrapper">
        <div 
          className={`tree-node ${isSelected ? 'selected' : ''}`}
          style={{ paddingLeft: `${depth * 16 + 8}px` }}
          onClick={() => {
            setSelectedNodeId(id);
            if (node.is_directory) toggleNode(id);
          }}
        >
          <span className="node-icon">
            {node.is_directory ? (
              isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />
            ) : (
              <FileText size={14} className="text-dim" />
            )}
          </span>
          <span className="node-type-icon">
            {node.is_directory ? <Folder size={14} className="text-accent" /> : <FileText size={14} className="text-primary" />}
          </span>
          <span className="node-name">{node.filename || (id === 5 ? "ROOT" : "UNKNOWN")}</span>
        </div>
        
        <AnimatePresence>
          {isExpanded && hasChildren && (
            <motion.div 
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="node-children"
            >
              {node.children.map(childId => renderTreeNode(childId, depth + 1))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  };

  const renderDriveSelection = () => (
    <div className="view-container drive-selection" dir="rtl">
      <div className="view-header">
        <h2>בחר כונן או מחיצה לסריקה:</h2>
        <button className="refresh-btn" onClick={loadDrives} disabled={isLoadingDrives}>
          <RefreshCw size={16} className={isLoadingDrives ? 'spin' : ''} />
          רענן רשימה
        </button>
      </div>

      {isLoadingDrives ? (
        <div className="loading-state">
          <RefreshCw size={48} className="spin text-dim" />
          <p>טוען חומרה מהמערכת...</p>
        </div>
      ) : (
        <div className="drives-grid">
          {/* כרטיסייה לטעינת אימג' מהמחשב */}
          <motion.div 
            className="drive-card image-load-card"
            whileHover={{ y: -4 }}
            whileTap={{ scale: 0.98 }}
            onClick={handleLoadImage}
          >
            <div className="drive-icon-wrapper text-accent">
              <FileArchive size={40} />
            </div>
            <div className="drive-info">
              <h3>טען קובץ אימג'</h3>
              <span className="drive-path">Forensic Image (.img, .dd)</span>
              <div className="drive-meta">
                <span className="drive-type">סריקת עותק דיגיטלי קיים</span>
              </div>
            </div>
            <button className="scan-now-btn secondary">בחר קובץ...</button>
          </motion.div>

          {drives.map(drive => (
            <motion.div 
              key={drive.id} 
              className={`drive-card ${drive.is_raw ? 'raw-warning' : ''}`}
              whileHover={{ y: -4 }}
              whileTap={{ scale: 0.98 }}
              style={{ border: drive.is_raw ? "1px solid var(--accent)" : "1px solid var(--border)" }}
            >
              <div className="drive-icon-wrapper">
                {drive.type_str.includes("USB") ? <Usb size={40} /> : <HardDrive size={40} />}
              </div>
              <div className="drive-info">
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <h3>{drive.name}</h3>
                  {drive.is_raw && <AlertTriangle size={16} className="text-accent" />}
                </div>
                <span className="drive-path">{drive.id}</span>
                
                <span style={{ 
                  color: drive.is_raw ? "var(--accent)" : "var(--text-dim)", 
                  fontSize: "13px", 
                  marginTop: "4px",
                  display: 'block',
                  fontWeight: drive.is_raw ? "600" : "normal"
                }}>
                  {drive.type_str}
                </span>

                <div style={{ 
                  backgroundColor: "rgba(0,0,0,0.2)", 
                  width: "100%", 
                  padding: "10px", 
                  borderRadius: "8px", 
                  marginTop: "12px", 
                  textAlign: "center",
                  border: "1px solid rgba(255,255,255,0.05)"
                }}>
                  <div style={{ color: "var(--text-dim)", fontSize: "11px", marginBottom: "2px" }}>סך הכל: {drive.size}</div>
                  <div style={{ color: drive.is_raw ? "var(--accent)" : "var(--success)", fontSize: "12px", fontWeight: 600 }}>{drive.used_space}</div>
                </div>

                {drive.is_raw && (
                  <p style={{ color: "var(--accent)", fontSize: "10px", marginTop: "8px", textAlign: 'center' }}>
                    ⚠️ מערכת קבצים פגומה זוהתה!
                  </p>
                )}

                {drive.is_raw && renderFixTools(drive)}
              </div>
              
              <div className="drive-card-actions" style={{ marginTop: 'auto', paddingTop: '16px' }}>
                <button className="scan-now-btn" onClick={() => startScan(drive.id)}>סרוק כונן</button>
                <button className="clone-btn" onClick={() => handleCreateImage(drive.id, drive.name)} title="צור אימג' (Clone)">
                  <Copy size={16} />
                </button>
              </div>
            </motion.div>
          ))}
          {drives.length === 0 && (
            <div className="empty-state">
              <AlertTriangle size={48} className="text-accent" />
              <p>לא נמצאו כוננים זמינים לסריקה.</p>
            </div>
          )}
        </div>
      )}
    </div>
  );

  const renderScanning = () => (
    <div className="view-container scanning-view" dir="rtl">
      <div className="scanning-content">
        <motion.div 
          className="scanning-icon"
          animate={{ rotate: 360 }}
          transition={{ duration: 2, repeat: Infinity, ease: "linear" }}
        >
          <RefreshCw size={80} className="text-primary" />
        </motion.div>
        
        <h2>סורק את {selectedDrive}...</h2>
        <p className="scanning-subtitle">נא לא לנתק את הכונן במהלך הפעולה</p>

        {progress && (
          <div className="progress-container">
            <div className="progress-stats">
              <span className="progress-percent">{progress.percentage.toFixed(1)}%</span>
              <span className="progress-found">נמצאו: {progress.files_found} קבצים</span>
            </div>
            
            <div className="progress-track">
              <motion.div 
                className="progress-fill"
                initial={{ width: 0 }}
                animate={{ width: `${progress.percentage}%` }}
              />
            </div>
            
            <div className="current-file-box">
              <label>קובץ נוכחי:</label>
              <p className="current-file-name">{progress.current_file}</p>
            </div>
          </div>
        )}
        
        <button className="stop-btn" onClick={() => setAppState("DRIVE_SELECTION")}>
          עצור סריקה
        </button>
      </div>
    </div>
  );

  const renderCloning = () => {
    if (!cloneProgress) return null;
    
    const copiedGB = (cloneProgress.copied_bytes / 1073741824).toFixed(2);
    const totalGB = (cloneProgress.total_bytes / 1073741824).toFixed(2);

    return (
      <div className="view-container scanning-view" dir="rtl">
        <div className="scanning-content">
          <motion.div 
            className="scanning-icon"
            animate={{ scale: [1, 1.1, 1] }}
            transition={{ duration: 2, repeat: Infinity }}
          >
            <Copy size={80} className="text-accent" />
          </motion.div>
          
          <h2>יוצר עותק פורנזי (Clone)...</h2>
          <p className="scanning-subtitle">מעתיק נתונים מ-{selectedDrive} לקובץ אימג'</p>
          
          <div className="progress-container">
            <div className="progress-stats">
              <span className="progress-percent">{cloneProgress.percentage.toFixed(1)}%</span>
              <span className="progress-found">{copiedGB} GB / {totalGB} GB</span>
            </div>
            
            <div className="progress-track">
              <motion.div 
                className="progress-fill"
                style={{ backgroundColor: 'var(--accent)' }}
                initial={{ width: 0 }}
                animate={{ width: `${cloneProgress.percentage}%` }}
              />
            </div>
            
            <div className="current-file-box">
              <label>סטטוס:</label>
              <p className="current-file-name">מעתיק בלוקים גולמיים (Raw Blocks)...</p>
            </div>
          </div>

          <button className="stop-btn" onClick={() => setAppState("DRIVE_SELECTION")}>
            בטל שכפול
          </button>
        </div>
      </div>
    );
  };

  const renderResults = () => {
    return (
      <div className="view-container results-view" dir="rtl">
        <header className="results-header">
          <button className="back-btn" onClick={() => setAppState("DRIVE_SELECTION")}>
            <ArrowLeft size={18} />
            חזור לבחירה
          </button>
          <div className="results-title">
            <h2>תוצאות סריקה: {selectedDrive}</h2>
            <span className="results-count">{progress?.files_found} קבצים נמצאו</span>
          </div>
          <div className="results-actions">
            <button 
              className={`recover-btn primary ${selectedFiles.size === 0 ? 'disabled' : ''}`}
              onClick={handleRecoverSelected}
              disabled={selectedFiles.size === 0}
            >
              <Download size={18} />
              שחזר נבחרים ({selectedFiles.size})
            </button>
          </div>
        </header>

        <div className="results-layout">
          <aside className="tree-sidebar">
            <div className="sidebar-header">
              <Folder size={16} />
              <span>עץ תיקיות</span>
            </div>
            <div className="tree-container" style={{ flex: 1 }}>
              {fileSystem[5] ? renderTreeNode(5) : <p className="empty-tree">טוען מבנה...</p>}
            </div>

            {/* אזור תצוגה מקדימה */}
            <div className="preview-panel" style={{ 
              borderTop: "1px solid var(--border)", 
              padding: "15px", 
              backgroundColor: "var(--bg-darker)",
              height: "250px",
              display: "flex",
              flexDirection: "column",
              gap: "10px"
            }}>
              <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-dim)", display: "flex", alignItems: "center", gap: "5px" }}>
                <Eye size={14} /> תצוגה מקדימה
              </div>
              <div style={{ 
                flex: 1, 
                backgroundColor: "rgba(0,0,0,0.2)", 
                borderRadius: "4px", 
                display: "flex", 
                alignItems: "center", 
                justifyContent: "center",
                overflow: "hidden",
                border: "1px dashed var(--border)"
              }}>
                {isLoadingPreview ? (
                  <div style={{ textAlign: "center" }}>
                    <Loader2 size={24} className="animate-spin text-primary" />
                    <p style={{ fontSize: "11px", marginTop: "5px" }}>טוען...</p>
                  </div>
                ) : previewUrl ? (
                  renderMediaPreview()
                ) : (
                  <div style={{ textAlign: "center", color: "var(--text-dim)", fontSize: "11px" }}>
                    בחר קובץ מדיה לתצוגה
                  </div>
                )}
              </div>
            </div>
          </aside>

          <main className="files-explorer">
            {/* נתיב (Breadcrumbs) כדי לחזור אחורה */}
            <div className="breadcrumbs" style={{ marginBottom: "15px", display: "flex", gap: "8px", fontSize: "14px" }}>
              {navigationHistory.map((nav, idx) => (
                <span key={nav.id} style={{ display: "flex", alignItems: "center", gap: "4px" }}>
                  <span 
                    style={{ cursor: "pointer", color: idx === navigationHistory.length - 1 ? "var(--primary)" : "var(--text-dim)" }} 
                    onClick={() => loadFolder(nav.id, nav.name)}
                  >
                    {nav.name}
                  </span>
                  {idx < navigationHistory.length - 1 && <ChevronRight size={14} className="text-dim" />}
                </span>
              ))}
            </div>

            <table className="files-table">
              <thead>
                <tr>
                  <th style={{ width: "40px" }}>
                    <input 
                      type="checkbox" 
                      onChange={(e) => {
                        if (e.target.checked) {
                          const allFileIds = folderContents.filter(f => !f.is_dir).map(f => f.mft_id);
                          setSelectedFiles(new Set(allFileIds));
                        } else {
                          setSelectedFiles(new Set());
                        }
                      }}
                      checked={folderContents.length > 0 && folderContents.filter(f => !f.is_dir).every(f => selectedFiles.has(f.mft_id))}
                    />
                  </th>
                  <th>שם קובץ</th>
                  <th>ID</th>
                  <th>סוג</th>
                  <th>גודל</th>
                  <th>פעולות</th>
                </tr>
              </thead>
              <tbody>
                {folderContents.length > 0 ? (
                  folderContents.map(item => (
                    <tr 
                      key={item.mft_id} 
                      onClick={() => setSelectedNodeId(item.mft_id)} 
                      className={selectedNodeId === item.mft_id ? 'active' : ''}
                      onDoubleClick={() => item.is_dir && loadFolder(item.mft_id, item.filename)}
                    >
                      <td>
                        {!item.is_dir && (
                          <input 
                            type="checkbox" 
                            checked={selectedFiles.has(item.mft_id)}
                            onClick={(e) => e.stopPropagation()}
                            onChange={(e) => {
                              const newSet = new Set(selectedFiles);
                              if (e.target.checked) newSet.add(item.mft_id);
                              else newSet.delete(item.mft_id);
                              setSelectedFiles(newSet);
                            }}
                          />
                        )}
                      </td>
                      <td>
                        <div className="file-name-cell">
                          {item.is_dir ? (
                            <Folder size={16} className="text-accent" />
                          ) : item.filename.toLowerCase().match(/\.(mp4|mkv|avi|mov|wmv)$/i) ? (
                            <Video size={16} className="text-primary" />
                          ) : item.filename.toLowerCase().match(/\.(mp3|wav|ogg|flac|m4a)$/i) ? (
                            <Music size={16} className="text-primary" />
                          ) : item.filename.toLowerCase().match(/\.(jpg|jpeg|png|gif|bmp|webp)$/i) ? (
                            <ImageIcon size={16} className="text-primary" />
                          ) : (
                            <FileText size={16} className="text-primary" />
                          )}
                          <span style={{ cursor: item.is_dir ? "pointer" : "default" }}>{item.filename}</span>
                        </div>
                      </td>
                      <td>{item.mft_id}</td>
                      <td>{item.is_dir ? "תיקייה" : "קובץ"}</td>
                      <td>{!item.is_dir ? `${(item.real_size / 1024).toFixed(2)} KB` : "-"}</td>
                      <td>
                        <div style={{ display: "flex", gap: "5px" }}>
                          {!item.is_dir && item.filename.toLowerCase().match(/\.(jpg|jpeg|png|gif|bmp|webp|mp4|mkv|avi|mov|wmv|mp3|wav|ogg|flac|m4a)$/i) && (
                            <button 
                              className="window-control-btn" 
                              style={{ width: "28px", height: "28px", borderRadius: "4px" }}
                              title="תצוגה מקדימה"
                              onClick={(e) => {
                                e.stopPropagation();
                                handlePreview(item);
                              }}
                            >
                              <Eye size={14} />
                            </button>
                          )}
                          {!item.is_dir && (
                            <button 
                              className="recover-btn primary" 
                              style={{ padding: '4px 12px', fontSize: '12px' }}
                              onClick={(e) => {
                                e.stopPropagation();
                                handleRecoverFile(item);
                              }}
                            >
                              שחזר ⬇️
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={6} className="empty-table">
                      תיקייה ריקה
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </main>
        </div>
      </div>
    );
  };

  if (!isAdmin) {
    return (
      <div className="app-shell">
        <header 
          data-tauri-drag-region 
          className="custom-titlebar"
          style={{ 
            backgroundColor: "var(--bg-darker)", 
            padding: "8px 15px", 
            borderBottom: "1px solid var(--border)", 
            display: "flex", 
            justifyContent: "space-between",
            alignItems: "center",
            userSelect: "none",
            height: "40px",
            zIndex: 1000
          }}
        >
          <div data-tauri-drag-region style={{ display: "flex", alignItems: "center", gap: "10px", pointerEvents: "none" }}>
            <span style={{ fontSize: "18px" }}>🦅</span>
            <h1 style={{ margin: 0, color: "var(--text-main)", fontSize: "14px", fontWeight: 600, letterSpacing: "0.5px" }}>
              PhoenixRecover <span style={{ color: "var(--primary)", fontSize: "11px", marginLeft: "4px" }}>PRO</span>
            </h1>
          </div>
          <div style={{ display: "flex", gap: "2px" }}>
            <button onClick={() => appWindow?.minimize()} className="window-control-btn" title="מזער">—</button>
            <button onClick={() => appWindow?.toggleMaximize()} className="window-control-btn" title="הגדל/הקטן">□</button>
            <button onClick={() => appWindow?.close()} className="window-control-btn close" title="סגור">✕</button>
          </div>
        </header>

        <div style={{ 
          padding: "50px", 
          textAlign: "center", 
          backgroundColor: "#1a0000", 
          flex: 1,
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "center",
          color: "white",
          fontFamily: "Inter, sans-serif"
        }}>
        <AlertTriangle size={80} color="#ff4444" style={{ marginBottom: "20px" }} />
        <h1 style={{ fontSize: "32px", marginBottom: "16px" }}>⚠️ אין הרשאות מנהל</h1>
        <p style={{ fontSize: "18px", maxWidth: "600px", lineHeight: "1.6", color: "#cccccc" }}>
          כדי לקרוא נתונים מכוננים פיזיים ולבצע פעולות שחזור, PhoenixRecover חייבת לרוץ עם הרשאות מנהל (Administrator).
        </p>
        <div style={{ 
          marginTop: "30px", 
          padding: "20px", 
          backgroundColor: "rgba(255,255,255,0.05)", 
          borderRadius: "12px",
          textAlign: "right"
        }} dir="rtl">
          <p><strong>איך מסדרים את זה?</strong></p>
          <ol style={{ marginTop: "10px", paddingRight: "20px" }}>
            <li>סגור את התוכנה לחלוטין.</li>
            <li>לחץ על האייקון של התוכנה עם המקש הימני בעכבר.</li>
            <li>בחר באפשרות <strong>"הפעל כמנהל" (Run as Administrator)</strong>.</li>
          </ol>
        </div>
        <button 
          onClick={() => window.location.reload()} 
          style={{ 
            marginTop: "40px", 
            padding: "12px 24px", 
            backgroundColor: "#ff4444", 
            border: "none", 
            borderRadius: "8px", 
            color: "white", 
            fontWeight: "bold",
            cursor: "pointer"
          }}
        >
          נסה שוב
        </button>
      </div>
    </div>
    );
  }

  return (
    <div className="app-shell">
      {/* שורת כותרת מותאמת אישית (Frameless Titlebar) */}
      <header 
        data-tauri-drag-region 
        className="custom-titlebar"
        style={{ 
          backgroundColor: "var(--bg-darker)", 
          padding: "8px 15px", 
          borderBottom: "1px solid var(--border)", 
          display: "flex", 
          justifyContent: "space-between",
          alignItems: "center",
          userSelect: "none",
          height: "40px",
          zIndex: 1000
        }}
      >
        {/* צד ימין - לוגו ושם התוכנה */}
        <div data-tauri-drag-region style={{ display: "flex", alignItems: "center", gap: "10px", pointerEvents: "none" }}>
          <span style={{ fontSize: "18px" }}>🦅</span>
          <h1 style={{ margin: 0, color: "var(--text-main)", fontSize: "14px", fontWeight: 600, letterSpacing: "0.5px" }}>
            PhoenixRecover <span style={{ color: "var(--primary)", fontSize: "11px", marginLeft: "4px" }}>PRO</span>
          </h1>
        </div>

        {/* צד שמאל - כפתורי שליטה (סגירה, הגדלה ומזעור) */}
        <div style={{ display: "flex", gap: "2px" }}>
          <button 
            onClick={() => appWindow?.minimize()} 
            className="window-control-btn"
            title="מזער"
          >
            —
          </button>
          <button 
            onClick={() => appWindow?.toggleMaximize()} 
            className="window-control-btn"
            title="הגדל/הקטן"
          >
            □
          </button>
          <button 
            onClick={() => appWindow?.close()} 
            className="window-control-btn close"
            title="סגור"
          >
            ✕
          </button>
        </div>
      </header>

      <nav className="top-bar">
        <div className="brand">
          <div className="brand-logo">🦅</div>
          <h1 className="brand-name">PhoenixRecover <span className="pro-tag">PRO</span></h1>
        </div>
        <div className="top-actions">
          <button className="icon-btn"><Search size={20} /></button>
          <button className="icon-btn" onClick={() => setIsSettingsOpen(true)}><SettingsIcon size={20} /></button>
        </div>
      </nav>

      <div className="main-viewport">
        {appState === "DRIVE_SELECTION" && renderDriveSelection()}
        {appState === "SCANNING" && renderScanning()}
        {appState === "CLONING" && renderCloning()}
        {appState === "RESULTS" && renderResults()}
      </div>

      <AnimatePresence>
        {isSettingsOpen && (
          <div className="modal-overlay" onClick={() => setIsSettingsOpen(false)}>
            <motion.div 
              className="modal-content"
              initial={{ scale: 0.9, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.9, opacity: 0 }}
              onClick={(e) => e.stopPropagation()}
              dir="rtl"
            >
              <div className="modal-header">
                <h2>הגדרות מערכת</h2>
                <button className="icon-btn" onClick={() => setIsSettingsOpen(false)}>
                  <X size={20} />
                </button>
              </div>

              <div className="settings-section">
                <label>עיצוב הממשק (Theme)</label>
                <div className="theme-options">
                  <button 
                    className={`theme-btn ${theme === 'light' ? 'active' : ''}`}
                    onClick={() => setTheme('light')}
                  >
                    <Sun size={20} />
                    <span>בהיר</span>
                  </button>
                  <button 
                    className={`theme-btn ${theme === 'dark' ? 'active' : ''}`}
                    onClick={() => setTheme('dark')}
                  >
                    <Moon size={20} />
                    <span>כהה</span>
                  </button>
                  <button 
                    className={`theme-btn ${theme === 'system' ? 'active' : ''}`}
                    onClick={() => setTheme('system')}
                  >
                    <Monitor size={20} />
                    <span>מערכת</span>
                  </button>
                </div>
              </div>

              <div className="settings-section">
                <label>גרסת תוכנה</label>
                <p style={{ fontSize: '13px', color: 'var(--text-dim)' }}>PhoenixRecover Pro v2.4.0 (Tauri Engine)</p>
              </div>

              <button className="close-modal-btn" onClick={() => setIsSettingsOpen(false)}>
                סגור ושמור
              </button>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default App;
