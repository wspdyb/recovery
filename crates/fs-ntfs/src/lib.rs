use core_io::{BlockReader, IOError};
use types::Artifact;
use async_trait::async_trait;
use thiserror::Error;
use serde::Serialize;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("Not supported")]
    NotSupported,
    #[error("Parse error")]
    ParseError,
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Self {
        Error::IoError(err.to_string())
    }
}

#[async_trait]
pub trait FilesystemParser {
    async fn is_supported<R: BlockReader + Send>(&self, reader: &mut R) -> Result<bool, Error>;
    async fn parse<R: BlockReader + Send>(&self, reader: &mut R) -> Result<Vec<Artifact>, Error>;
}

pub struct NtfsParser;

const MFT_RECORD_SIZE: usize = 1024;
const ATTR_FILE_NAME: u32 = 0x30;
const ATTR_DATA: u32 = 0x80;

#[derive(Debug, Clone, Serialize)]
pub struct DataRun {
    pub cluster_offset: i64, // LCN המוחלט
    pub cluster_count: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum DataInfo {
    Resident(Vec<u8>),
    NonResident {
        allocated_size: u64,
        real_size: u64,
        data_runs: Vec<DataRun>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedRecord {
    pub mft_id: u64,
    pub parent_mft_id: u64,
    pub filename: Option<String>,
    pub is_directory: bool,
    pub is_deleted: bool, // NEW
    pub data_info: Option<DataInfo>,
}

// לחלץ את הנתונים הקריטיים מהסקטור הראשון של המחיצה (VBR)
pub struct NtfsBootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub mft_cluster: u64,
}

impl NtfsBootSector {
    pub fn parse(boot_sector: &[u8]) -> Result<Self, Error> {
        if boot_sector.len() < 512 {
            return Err(Error::ParseError);
        }

        // קריאת הנתונים בעזרת from_le_bytes (Little Endian)
        let bytes_per_sector = u16::from_le_bytes([boot_sector[11], boot_sector[12]]);
        let sectors_per_cluster = boot_sector[13];

        let mut mft_bytes = [0u8; 8];
        mft_bytes.copy_from_slice(&boot_sector[48..56]);
        let mft_cluster = u64::from_le_bytes(mft_bytes);

        Ok(Self {
            bytes_per_sector,
            sectors_per_cluster,
            mft_cluster,
        })
    }

    // חישוב המיקום המדויק (בבתים) מתחילת המחיצה אל התחלת ה-MFT
    pub fn mft_offset(&self) -> u64 {
        self.mft_cluster * (self.sectors_per_cluster as u64) * (self.bytes_per_sector as u64)
    }
}

fn decode_utf16(bytes: &[u8]) -> Option<String> {
    let u16_data: Vec<u16> = bytes.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&u16_data).ok()
}

pub fn parse_data_runs(runlist: &[u8]) -> Vec<DataRun> {
    let mut runs = Vec::new();
    let mut current_idx = 0;
    let mut absolute_cluster: i64 = 0;

    while current_idx < runlist.len() {
        let header = runlist[current_idx];
        if header == 0x00 { break; }

        current_idx += 1;
        let length_bytes_count = (header & 0x0F) as usize;
        let offset_bytes_count = ((header & 0xF0) >> 4) as usize;

        if current_idx + length_bytes_count + offset_bytes_count > runlist.len() {
            break;
        }

        // 1. קריאת אורך ה-Run
        let mut length_buf = [0u8; 8];
        for i in 0..length_bytes_count {
            length_buf[i] = runlist[current_idx + i];
        }
        let cluster_count = u64::from_le_bytes(length_buf);
        current_idx += length_bytes_count;

        // 2. קריאת ההיסט (Relative Offset)
        let mut offset_buf = [0u8; 8];
        if offset_bytes_count > 0 {
            for i in 0..offset_bytes_count {
                offset_buf[i] = runlist[current_idx + i];
            }

            // Sign extension
            if (runlist[current_idx + offset_bytes_count - 1] & 0x80) != 0 {
                for i in offset_bytes_count..8 { offset_buf[i] = 0xFF; }
            }

            let relative_offset = i64::from_le_bytes(offset_buf);
            absolute_cluster += relative_offset;
            current_idx += offset_bytes_count;
        }

        runs.push(DataRun {
            cluster_offset: absolute_cluster,
            cluster_count,
        });
    }

    runs
}

pub fn parse_mft_record(buffer: &[u8], mft_id: u64) -> ParsedRecord {
    let mut result = ParsedRecord {
        mft_id,
        parent_mft_id: 0,
        filename: None,
        is_directory: false,
        is_deleted: false,
        data_info: None,
    };

    if buffer.len() < 42 || &buffer[0..4] != b"FILE" {
        return result;
    }

    let flags = u16::from_le_bytes([buffer[22], buffer[23]]);
    result.is_deleted = (flags & 0x01) == 0;

    let mut attr_offset = u16::from_le_bytes([buffer[20], buffer[21]]) as usize;

    while attr_offset + 8 <= buffer.len() {
        let attr_type = u32::from_le_bytes([
            buffer[attr_offset], buffer[attr_offset+1],
            buffer[attr_offset+2], buffer[attr_offset+3]
        ]);

        if attr_type == 0xFFFFFFFF {
            break;
        }

        let attr_length = u32::from_le_bytes([
            buffer[attr_offset+4], buffer[attr_offset+5],
            buffer[attr_offset+6], buffer[attr_offset+7]
        ]) as usize;

        if attr_length == 0 || attr_offset + attr_length > buffer.len() {
            break;
        }

        if attr_type == ATTR_FILE_NAME {
            if attr_offset + 22 <= buffer.len() {
                let content_offset_rel = u16::from_le_bytes([buffer[attr_offset+20], buffer[attr_offset+21]]) as usize;
                let content_offset = attr_offset + content_offset_rel;

                if content_offset + 66 <= buffer.len() {
                    // 1. חילוץ מזהה תיקיית האב (6 בתים ראשונים מתוך ה-8)
                    let mut parent_bytes = [0u8; 8];
                    parent_bytes.copy_from_slice(&buffer[content_offset..content_offset+8]);
                    result.parent_mft_id = u64::from_le_bytes(parent_bytes) & 0x0000FFFFFFFFFFFF;

                    // 2. בדיקה האם זו תיקייה (דגל 0x10000000 באופסט 56)
                    let file_flags = u32::from_le_bytes([
                        buffer[content_offset+56], buffer[content_offset+57],
                        buffer[content_offset+58], buffer[content_offset+59]
                    ]);
                    result.is_directory = (file_flags & 0x10000000) != 0;

                    // 3. חילוץ השם
                    let name_length = buffer[content_offset + 64] as usize;
                    let name_start = content_offset + 66;
                    let name_end = name_start + (name_length * 2);

                    if name_end <= buffer.len() {
                        result.filename = decode_utf16(&buffer[name_start..name_end]);
                    }
                }
            }
        } else if attr_type == ATTR_DATA {
            let non_resident_flag = buffer[attr_offset + 8];
            if non_resident_flag == 0 {
                // Resident
                if attr_offset + 22 <= buffer.len() {
                    let content_length = u32::from_le_bytes([
                        buffer[attr_offset+16], buffer[attr_offset+17],
                        buffer[attr_offset+18], buffer[attr_offset+19]
                    ]) as usize;
                    let content_offset_rel = u16::from_le_bytes([
                        buffer[attr_offset+20], buffer[attr_offset+21]
                    ]) as usize;
                    let content_offset = attr_offset + content_offset_rel;

                    if content_offset + content_length <= buffer.len() {
                        result.data_info = Some(DataInfo::Resident(buffer[content_offset..content_offset + content_length].to_vec()));
                    }
                }
            } else {
                // Non-Resident
                if attr_offset + 56 <= buffer.len() {
                    let runlist_offset_rel = u16::from_le_bytes([
                        buffer[attr_offset+32], buffer[attr_offset+33]
                    ]) as usize;
                    let runlist_offset = attr_offset + runlist_offset_rel;

                    let allocated_size = u64::from_le_bytes(buffer[attr_offset+40..attr_offset+48].try_into().unwrap());
                    let real_size = u64::from_le_bytes(buffer[attr_offset+48..attr_offset+56].try_into().unwrap());

                    if runlist_offset < attr_offset + attr_length {
                        let runs = parse_data_runs(&buffer[runlist_offset..attr_offset + attr_length]);
                        result.data_info = Some(DataInfo::NonResident {
                            allocated_size,
                            real_size,
                            data_runs: runs,
                        });
                    }
                }
            }
        }

        attr_offset += attr_length;
    }

    result
}

pub async fn recover_file<R: BlockReader + Send>(
    reader: &mut R,
    data_info: &DataInfo,
    bytes_per_cluster: u64,
    output_path: &std::path::Path,
) -> Result<(), Error> {
    use tokio::io::AsyncWriteExt;

    let mut out_file = tokio::fs::File::create(output_path).await.map_err(|e: std::io::Error| Error::IoError(e.to_string()))?;

    match data_info {
        DataInfo::Resident(data) => {
            out_file.write_all(data).await.map_err(|e: std::io::Error| Error::IoError(e.to_string()))?;
        }
        DataInfo::NonResident { real_size, data_runs, .. } => {
            let mut bytes_written = 0;

            for run in data_runs {
                let run_offset_bytes = (run.cluster_offset as u64) * bytes_per_cluster;
                let run_size_bytes = run.cluster_count * bytes_per_cluster;

                let run_data = reader.read_at(run_offset_bytes, run_size_bytes as usize).await?;

                let remaining_bytes = *real_size - bytes_written;
                let bytes_to_write = std::cmp::min(run_data.len() as u64, remaining_bytes) as usize;

                out_file.write_all(&run_data[..bytes_to_write]).await.map_err(|e: std::io::Error| Error::IoError(e.to_string()))?;
                bytes_written += bytes_to_write as u64;

                if bytes_written >= *real_size {
                    break;
                }
            }
        }
    }

    Ok(())
}

impl NtfsParser {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FilesystemParser for NtfsParser {
    async fn is_supported<R: BlockReader + Send>(&self, reader: &mut R) -> Result<bool, Error> {
        let buffer = reader.read_at(0, 512).await?;

        if buffer.len() >= 11 && &buffer[3..7] == b"NTFS" {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn parse<R: BlockReader + Send>(&self, reader: &mut R) -> Result<Vec<Artifact>, Error> {
        let boot_sector_data = reader.read_at(0, 512).await?;
        let boot_sector = NtfsBootSector::parse(&boot_sector_data)?;

        let mut artifacts = Vec::new();
        let mut current_offset = boot_sector.mft_offset();

        // Scan first 1000 records for MVP
        for i in 0..1000 {
            if let Ok(record_bytes) = reader.read_at(current_offset, MFT_RECORD_SIZE).await {
                if record_bytes.len() == MFT_RECORD_SIZE && &record_bytes[0..4] == b"FILE" {
                    let record = parse_mft_record(&record_bytes, i as u64);
                    if let Some(filename) = record.filename {
                        artifacts.push(Artifact {
                            id: None,
                            session_id: 0,
                            artifact_type: types::ArtifactType::File,
                            size_bytes: None,
                            offset_start: Some(current_offset as i64),
                            offset_end: Some((current_offset + MFT_RECORD_SIZE as u64) as i64),
                            confidence: 1.0.into(),
                            status: filename,
                        });
                    }
                }
            }
            current_offset += MFT_RECORD_SIZE as u64;
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_io::SourceInfo;
    use std::path::PathBuf;

    struct MockReader {
        data: Vec<u8>,
        source_info: SourceInfo,
    }

    impl MockReader {
        fn new(data: Vec<u8>) -> Self {
            Self {
                source_info: SourceInfo {
                    path: PathBuf::from("mock"),
                    total_size: data.len() as u64,
                    block_size: 512,
                    read_only: true,
                },
                data,
            }
        }
    }

    #[async_trait]
    impl BlockReader for MockReader {
        async fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError> {
            if offset as usize >= self.data.len() {
                return Ok(Vec::new());
            }
            let end = std::cmp::min((offset as usize) + size, self.data.len());
            Ok(self.data[(offset as usize)..end].to_vec())
        }

        fn source_info(&self) -> &SourceInfo {
            &self.source_info
        }
    }

    #[tokio::test]
    async fn test_ntfs_is_supported_valid() {
        let mut bpb = vec![0u8; 512];
        bpb[3..7].copy_from_slice(b"NTFS");

        let mut reader = MockReader::new(bpb);
        let parser = NtfsParser::new();
        let result = parser.is_supported(&mut reader).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_ntfs_is_supported_invalid() {
        let bpb = vec![0u8; 512]; // all zero
        let mut reader = MockReader::new(bpb);
        let parser = NtfsParser::new();
        let result = parser.is_supported(&mut reader).await.unwrap();
        assert!(!result);
    }
}
