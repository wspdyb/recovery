use core_io::BlockReader;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileCategory {
    Image,
    Video,
    Audio,
    Archive,
    Document,
    AndroidApp,
    Executable,
    Database,
    Data,
    Forensic,
    Code,
    Resource,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct CarvedFile {
    pub offset: u64,
    pub file_type: String,
    pub extension: String,
    pub mime_type: String, // שדה חדש לתמיכה בהזרמה
    pub estimated_size: u64,
    pub entropy: f32,
    pub category: FileCategory,
}

#[derive(Deserialize, Clone, Debug)]
pub struct FileSignature {
    pub name: String,
    pub extension: String,
    pub magic: Vec<u8>,
    pub offset: usize,
    pub mime: String, // שדה חדש מה-JSON
    pub footer: Option<Vec<u8>>,
    pub default_size: Option<u64>,
    pub category: FileCategory,
}

pub struct SignatureEngine {
    pub signatures: Vec<FileSignature>,
}

pub fn calculate_entropy(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    let mut entropy = 0.0;
    let len = data.len() as f32;
    for &count in &counts {
        if count > 0 {
            let p = count as f32 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

impl SignatureEngine {
    pub fn load_from_json(json: &str) -> Result<Self, String> {
        let signatures: Vec<FileSignature> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse signatures JSON: {}", e))?;
        Ok(Self { signatures })
    }

    pub fn identify_smart(&self, data: &[u8]) -> (String, String, String, FileCategory) {
        for sig in &self.signatures {
            if data.len() >= sig.offset + sig.magic.len() {
                if &data[sig.offset..sig.offset + sig.magic.len()] == sig.magic.as_slice() {
                    return (sig.name.clone(), sig.extension.clone(), sig.mime.clone(), sig.category.clone());
                }
            }
        }

        let entropy = calculate_entropy(data);
        if entropy > 7.5 {
            return (
                "High Entropy Data (Encrypted/Compressed)".into(),
                "bin".into(),
                "application/octet-stream".into(),
                FileCategory::Unknown,
            );
        }

        (
            "Unknown Data".into(),
            "bin".into(),
            "application/octet-stream".into(),
            FileCategory::Unknown,
        )
    }
}

// פונקציה לסריקה גולמית של כונן
pub async fn carve_drive<R: BlockReader + Send>(
    reader: &mut R,
    start_offset: u64,
    end_offset: u64,
    engine: &SignatureEngine,
) -> Result<Vec<CarvedFile>, String> {

    let mut found_files = Vec::new();
    let chunk_size: usize = 1024 * 1024; // 1MB בכל קריאה לביצועים טובים
    let mut current_offset = start_offset;

    println!("מתחיל Carving דינמי מ-{} עד {}", start_offset, end_offset);

    while current_offset < end_offset {
        let read_size = std::cmp::min(chunk_size as u64, end_offset - current_offset) as usize;
        let buffer = reader.read_at(current_offset, read_size).await
            .map_err(|e| format!("Read error at {}: {}", current_offset, e))?;

        // סריקה של כל byte בחיפוש אחר חתימות
        for i in 0..buffer.len().saturating_sub(16) {
            let (name, ext, mime, cat) = engine.identify_smart(&buffer[i..]);
            if name != "Unknown Data" {
                found_files.push(CarvedFile {
                    offset: current_offset + i as u64,
                    file_type: name,
                    extension: ext,
                    mime_type: mime,
                    estimated_size: 4096, // default for carver
                    entropy: calculate_entropy(&buffer[i..std::cmp::min(i+4096, buffer.len())]),
                    category: cat,
                });
            } else {
                // בדיקת אנטרופיה לנתונים מוצפנים ללא חתימה
                if i % 4096 == 0 {
                    let sample = &buffer[i..std::cmp::min(i+512, buffer.len())];
                    let entropy = calculate_entropy(sample);
                    if entropy > 7.5 {
                        found_files.push(CarvedFile {
                            offset: current_offset + i as u64,
                            file_type: "High Entropy Data (Encrypted/Compressed)".to_string(),
                            extension: "bin".to_string(),
                            mime_type: "application/octet-stream".to_string(),
                            estimated_size: 4096,
                            entropy,
                            category: FileCategory::Unknown,
                        });
                    }
                }
            }
        }
        current_offset += read_size as u64;
    }

    Ok(found_files)
}
