use std::path::{Path, PathBuf};
use async_trait::async_trait;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub path: PathBuf,
    pub total_size: u64,
    pub block_size: u64,
    pub read_only: bool,
}

pub const MAX_READ_SIZE: usize = 100 * 1024 * 1024; // 100MB

#[derive(Debug, Error)]
pub enum IOError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid block size: {0}")]
    InvalidBlockSize(u64),

    #[error("read size too large: {0} (max: {})", MAX_READ_SIZE)]
    ReadSizeTooLarge(usize),

    #[error("read out of bounds: offset={offset}, size={size}, total={total}")]
    OutOfBounds {
        offset: u64,
        size: usize,
        total: u64,
    },

    #[error("numeric conversion failed")]
    NumericConversion,
}

#[async_trait]
pub trait BlockReader: Send {
    async fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError>;
    fn source_info(&self) -> &SourceInfo;
    fn current_offset(&self) -> u64;

    fn total_size(&self) -> u64 {
        self.source_info().total_size
    }

    fn block_size(&self) -> u64 {
        self.source_info().block_size
    }
}

pub struct FileBlockReader {
    file: File,
    info: SourceInfo,
    current_pos: u64,
}

impl FileBlockReader {
    pub async fn open(path: impl AsRef<Path>, block_size: u64) -> Result<Self, IOError> {
        if block_size == 0 {
            return Err(IOError::InvalidBlockSize(block_size));
        }

        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf).await?;
        let metadata = tokio::fs::metadata(&path_buf).await?;

        Ok(Self {
            file,
            info: SourceInfo {
                path: path_buf,
                total_size: metadata.len(),
                block_size,
                read_only: true,
            },
            current_pos: 0,
        })
    }

    pub async fn read_exact_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError> {
        self.ensure_bounds(offset, size)?;
        self.file.seek(SeekFrom::Start(offset)).await?;

        let mut buf = vec![0u8; size];
        self.file.read_exact(&mut buf).await?;
        Ok(buf)
    }

    fn ensure_bounds(&self, offset: u64, size: usize) -> Result<(), IOError> {
        if size > MAX_READ_SIZE {
            return Err(IOError::ReadSizeTooLarge(size));
        }

        let size_u64 = u64::try_from(size).map_err(|_| IOError::NumericConversion)?;
        let end = offset.checked_add(size_u64).ok_or(IOError::NumericConversion)?;

        if end > self.info.total_size {
            return Err(IOError::OutOfBounds {
                offset,
                size,
                total: self.info.total_size,
            });
        }

        Ok(())
    }
}

#[async_trait]
impl BlockReader for FileBlockReader {
    async fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError> {
        self.ensure_bounds(offset, size)?;
        self.file.seek(SeekFrom::Start(offset)).await?;

        let mut buf = vec![0u8; size];
        self.file.read_exact(&mut buf).await?;
        self.current_pos = offset + size as u64;
        Ok(buf)
    }

    fn source_info(&self) -> &SourceInfo {
        &self.info
    }

    fn current_offset(&self) -> u64 {
        self.current_pos
    }
}

pub struct PartitionBlockReader {
    inner: Box<dyn BlockReader>,
    start_offset: u64,
    size: u64,
    current_pos: u64,
    info: SourceInfo,
}

impl PartitionBlockReader {
    pub fn new(mut inner: Box<dyn BlockReader>, start_offset: u64, size: u64) -> Self {
        let mut info = inner.source_info().clone();
        info.total_size = size;
        
        Self {
            inner,
            start_offset,
            size,
            current_pos: 0,
            info,
        }
    }
}

#[async_trait]
impl BlockReader for PartitionBlockReader {
    async fn read_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IOError> {
        if offset + size as u64 > self.size {
            return Err(IOError::OutOfBounds {
                offset,
                size,
                total: self.size,
            });
        }

        let actual_offset = self.start_offset + offset;
        let data = self.inner.read_at(actual_offset, size).await?;
        self.current_pos = offset + data.len() as u64;
        Ok(data)
    }

    fn source_info(&self) -> &SourceInfo {
        &self.info
    }

    fn current_offset(&self) -> u64 {
        self.current_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    fn temp_file_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    #[tokio::test]
    async fn file_reader_reads_range() {
        let path = temp_file_path("phoenixrecover_core_io_test_1.bin");
        let mut f = File::create(&path).await.unwrap();
        f.write_all(b"abcdefghij").await.unwrap();
        f.flush().await.unwrap();
        drop(f);

        let mut reader = FileBlockReader::open(&path, 512).await.unwrap();
        let data = reader.read_at(2, 4).await.unwrap();

        assert_eq!(data, b"cdef");

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn file_reader_read_exact_at_reads_exact_size() {
        let path = temp_file_path("phoenixrecover_core_io_test_2.bin");
        let mut f = File::create(&path).await.unwrap();
        f.write_all(b"0123456789").await.unwrap();
        f.flush().await.unwrap();
        drop(f);

        let mut reader = FileBlockReader::open(&path, 512).await.unwrap();
        let data = reader.read_exact_at(5, 3).await.unwrap();

        assert_eq!(data, b"567");

        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn file_reader_rejects_out_of_bounds() {
        let path = temp_file_path("phoenixrecover_core_io_test_3.bin");
        let mut f = File::create(&path).await.unwrap();
        f.write_all(b"12345").await.unwrap();
        f.flush().await.unwrap();
        drop(f);

        let mut reader = FileBlockReader::open(&path, 512).await.unwrap();
        let err = reader.read_at(4, 4).await.unwrap_err();

        match err {
            IOError::OutOfBounds { .. } => {}
            other => panic!("unexpected error: {other:?}"),
        }

        let _ = tokio::fs::remove_file(path).await;
    }
}
