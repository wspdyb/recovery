use core_io::{BlockReader, IOError};

#[derive(Debug, PartialEq)]
pub enum PartitionLayout {
    MBR(Vec<PartitionEntry>),
    GPT(Vec<PartitionEntry>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionEntry {
    pub start_lba: u64,
    pub sector_count: u64,
    pub partition_type: String, // קוד מחיצה (למשל 0x07 ל-NTFS ב-MBR)
}

pub async fn detect_layout<R: BlockReader + Send>(reader: &mut R) -> Result<PartitionLayout, IOError> {
    // קריאת הסקטור הראשון (LBA 0) שבו יושב ה-MBR (ולפעמים ה-Protective MBR של ה-GPT)
    let sector_0 = reader.read_at(0, 512).await?;
    
    if sector_0.len() < 512 {
        return Ok(PartitionLayout::Unknown);
    }

    // בדיקת חתימת MBR (בייט 510 ו-511 צריכים להיות 0x55 ו-0xAA)
    if sector_0[510] == 0x55 && sector_0[511] == 0xAA {
        // קריאת סקטור 1 (LBA 1) כדי לבדוק אם יש שם GPT Header
        if let Ok(sector_1) = reader.read_at(512, 512).await {
            if sector_1.len() >= 8 && &sector_1[0..8] == b"EFI PART" {
                // זהו כונן GPT
                return Ok(PartitionLayout::GPT(parse_gpt_entries(reader).await?));
            }
        }
        
        // אם אין GPT אבל יש חתימת 55 AA, זהו MBR רגיל
        return Ok(PartitionLayout::MBR(parse_mbr_entries(&sector_0)?));
    }
    
    Ok(PartitionLayout::Unknown)
}

// פונקציות עזר שמפרסרות את הביטים עצמם מתוך הטבלה
fn parse_mbr_entries(sector_0: &[u8]) -> Result<Vec<PartitionEntry>, IOError> {
    let mut entries = Vec::new();
    // טבלת מחיצות MBR מתחילה באופסט 446 (4 רשומות של 16 בתים כל אחת)
    for i in 0..4 {
        let offset = 446 + (i * 16);
        let entry_bytes = &sector_0[offset..offset + 16];
        
        let p_type = entry_bytes[4];
        if p_type == 0 {
            continue; // Empty entry
        }

        let start_lba = u32::from_le_bytes([
            entry_bytes[8], entry_bytes[9], entry_bytes[10], entry_bytes[11]
        ]) as u64;
        
        let sector_count = u32::from_le_bytes([
            entry_bytes[12], entry_bytes[13], entry_bytes[14], entry_bytes[15]
        ]) as u64;

        entries.push(PartitionEntry {
            start_lba,
            sector_count,
            partition_type: format!("0x{:02X}", p_type),
        });
    }
    Ok(entries)
}

async fn parse_gpt_entries<R: BlockReader + Send>(_reader: &mut R) -> Result<Vec<PartitionEntry>, IOError> {
    // כאן תבוא לוגיקה לקריאת טבלת ה-GPT (לרוב ב-LBA 2)
    Ok(Vec::new())
}

// סוגי מערכות הקבצים שאנחנו יודעים לזהות מתוך הברזל
#[derive(Debug, Clone)]
pub enum FileSystemType {
    NTFS,
    FAT32,
    EXFAT,
    EXT4,
    UnknownRaw,
}

impl FileSystemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileSystemType::NTFS => "NTFS",
            FileSystemType::FAT32 => "FAT32",
            FileSystemType::EXFAT => "exFAT",
            FileSystemType::EXT4 => "ext4",
            FileSystemType::UnknownRaw => "RAW (לא מוכר)",
        }
    }
}

// פונקציה שמקבלת את סקטור 0 (512 בתים) של המחיצה ומחזירה את סוגה
pub fn detect_file_system(boot_sector: &[u8]) -> FileSystemType {
    if boot_sector.len() < 512 {
        return FileSystemType::UnknownRaw;
    }

    let signature = &boot_sector[3..11];
    
    if signature == b"NTFS    " {
        return FileSystemType::NTFS;
    } else if signature == b"EXFAT   " {
        return FileSystemType::EXFAT;
    } else if signature == b"MSDOS5.0" || signature == b"mkfs.fat" {
        // FAT32 לרוב משתמשת בחתימות האלה, או שיש לה "FAT32   " באופסט 82
        return FileSystemType::FAT32;
    } 
    
    // בדיקת FAT32 ספציפית באופסט 82
    if boot_sector.len() >= 90 && &boot_sector[82..90] == b"FAT32   " {
        return FileSystemType::FAT32;
    }

    // (Ext4 דורש קריאה של אופסט 1024, אז נדלג עליו כרגע בבדיקה המהירה של סקטור 0)

    FileSystemType::UnknownRaw
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_io::SourceInfo;
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct MockReader {
        data: Vec<u8>,
        info: SourceInfo,
    }

    impl MockReader {
        fn new(data: Vec<u8>) -> Self {
            let size = data.len() as u64;
            Self {
                data,
                info: SourceInfo {
                    path: PathBuf::from("mock"),
                    total_size: size,
                    block_size: 512,
                    read_only: true,
                },
            }
        }
    }

    #[async_trait]
    impl BlockReader for MockReader {
        async fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError> {
            let start = offset as usize;
            let end = start + size;
            if end > self.data.len() {
                return Err(IOError::OutOfBounds { offset, size, total: self.data.len() as u64 });
            }
            Ok(self.data[start..end].to_vec())
        }
        fn source_info(&self) -> &SourceInfo { &self.info }
        fn current_offset(&self) -> u64 { 0 }
    }

    #[tokio::test]
    async fn test_detect_mbr() {
        let mut data = vec![0u8; 1024];
        data[510] = 0x55;
        data[511] = 0xAA;
        
        // Entry 1: NTFS, Start 2048, Size 10000
        data[446 + 4] = 0x07;
        data[446 + 8..446 + 12].copy_from_slice(&2048u32.to_le_bytes());
        data[446 + 12..446 + 16].copy_from_slice(&10000u32.to_le_bytes());

        let mut reader = MockReader::new(data);
        let layout = detect_layout(&mut reader).await.unwrap();
        
        if let PartitionLayout::MBR(entries) = layout {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].partition_type, "0x07");
            assert_eq!(entries[0].start_lba, 2048);
            assert_eq!(entries[0].sector_count, 10000);
        } else {
            panic!("Expected MBR");
        }
    }
}

