# חוזי API — ממשקים בין מודולים

גרסה: 1.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית

## מבוא

מסמך זה מגדיר את החוזים הטכניים בין מודולים במערכת. כל חוזה כולל traits, structs, וכללי שימוש.

## 1. Core Types

### Shared Enums

```rust
#[derive(Serialize, Deserialize)]
pub enum SourceType {
    RawDevice(String), // path to device
    ImageFile(String),  // path to image
    Partition { device: String, start: u64, size: u64 },
}

#[derive(Serialize, Deserialize)]
pub enum ArtifactType {
    File,
    Directory,
    Fragment,
}

#[derive(Serialize, Deserialize)]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Partial,
}

#[derive(Serialize, Deserialize)]
pub enum ScoreGrade {
    A, B, C, D, F,
}
```

### Shared Structs

```rust
#[derive(Serialize, Deserialize)]
pub struct Artifact {
    pub id: Option<i64>,
    pub session_id: i64,
    pub artifact_type: ArtifactType,
    pub size_bytes: Option<i64>,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub confidence: f64,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct ValidationResult {
    pub artifact_id: i64,
    pub validator_type: String,
    pub status: ValidationStatus,
    pub notes: Option<String>,
    pub repair_hints: Option<String>,
    pub preview_metadata: Option<Value>, // JSON
}
```

## 2. Core-IO Contract

```rust
pub trait BlockReader {
    async fn read_block(&self, offset: u64, size: usize) -> Result<Vec<u8>, IOError>;
    fn total_blocks(&self) -> u64;
    fn block_size(&self) -> usize;
}

pub trait AsyncReadSeek: AsyncRead + AsyncSeek {}

pub struct ReadConfig {
    pub buffer_size: usize,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
    pub checksum_verify: bool,
}
```

## 3. Acquisition Contract

```rust
pub trait AcquisitionSource {
    fn source_type(&self) -> SourceType;
    fn size_bytes(&self) -> u64;
    fn block_size(&self) -> usize;
    async fn acquire(&self, config: &ReadConfig) -> Result<Box<dyn BlockReader>, AcquisitionError>;
}

pub struct AcquisitionResult {
    pub reader: Box<dyn BlockReader>,
    pub metadata: SourceMetadata,
}

pub struct SourceMetadata {
    pub checksum: Option<String>,
    pub bad_sectors: Vec<u64>,
}
```

## 4. Storage Abstraction Contract

```rust
pub trait StorageAbstraction {
    fn get_reader(&self) -> Result<Box<dyn BlockReader>, StorageError>;
    fn metadata(&self) -> &StorageMetadata;
}

pub struct StorageMetadata {
    pub total_size: u64,
    pub block_size: usize,
    pub storage_type: String,
}
```

## 5. Disk Layout Contract

```rust
pub trait DiskLayoutParser {
    fn detect_layout(&self, reader: &dyn BlockReader) -> Result<DiskLayout, LayoutError>;
}

#[derive(Serialize, Deserialize)]
pub struct DiskLayout {
    pub layout_type: LayoutType, // MBR, GPT
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct PartitionInfo {
    pub start_sector: u64,
    pub sector_count: u64,
    pub partition_type: String,
    pub name: Option<String>,
}
```

## 6. Filesystem Parsers Contract

```rust
pub trait FilesystemParser {
    fn filesystem_type(&self) -> &'static str;
    async fn parse(&self, reader: &dyn BlockReader, layout: &DiskLayout) -> Result<FilesystemMetadata, ParseError>;
}

#[derive(Serialize, Deserialize)]
pub struct FilesystemMetadata {
    pub root: DirectoryEntry,
    pub files: Vec<FileEntry>,
    pub orphans: Vec<FileEntry>,
    pub deleted: Vec<FileEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mtime: DateTime<Utc>,
    pub offset: Option<u64>,
    pub is_deleted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub path: String,
    pub children: Vec<FileEntry>,
}
```

## 7. Metadata Recovery Contract

```rust
pub trait MetadataRecovery {
    async fn recover(&self, fs_metadata: &FilesystemMetadata) -> Result<RecoveredMetadata, RecoveryError>;
}

#[derive(Serialize, Deserialize)]
pub struct RecoveredMetadata {
    pub recovered_files: Vec<RecoveredFile>,
    pub orphan_files: Vec<RecoveredFile>,
    pub directory_tree: DirectoryTree,
}

#[derive(Serialize, Deserialize)]
pub struct RecoveredFile {
    pub original_entry: FileEntry,
    pub recovered_path: Option<String>,
    pub confidence: f64,
}
```

## 8. Carving Contract

```rust
pub trait Carver {
    fn signatures(&self) -> &[CarveSignature];
    async fn carve(&self, reader: &dyn BlockReader) -> Result<Vec<CarvedArtifact>, CarveError>;
}

#[derive(Serialize, Deserialize)]
pub struct CarveSignature {
    pub name: String,
    pub header: Vec<u8>,
    pub footer: Option<Vec<u8>>,
    pub max_size: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct CarvedArtifact {
    pub offset: u64,
    pub size: usize,
    pub signature: String,
    pub data: Vec<u8>,
}
```

## 9. Fragment Reconstruction Contract

```rust
pub trait FragmentReconstructor {
    async fn reconstruct(&self, fragments: &[CarvedArtifact]) -> Result<Vec<ReconstructedFile>, ReconstructionError>;
}

#[derive(Serialize, Deserialize)]
pub struct ReconstructedFile {
    pub fragments: Vec<CarvedArtifact>,
    pub assembled_data: Vec<u8>,
    pub confidence: f64,
}
```

## 10. Validation Contract

```rust
pub trait Validator {
    fn supported_types(&self) -> &[&'static str];
    async fn validate(&self, data: &[u8]) -> Result<ValidationResult, ValidationError>;
}
```

## 11. Dedup + Fusion Contract

```rust
pub trait DedupFusion {
    async fn dedup(&self, artifacts: &[Artifact]) -> Result<DedupResult, DedupError>;
    async fn fuse(&self, groups: &[ArtifactGroup]) -> Result<FusionResult, FusionError>;
}

#[derive(Serialize, Deserialize)]
pub struct ArtifactGroup {
    pub artifacts: Vec<Artifact>,
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct DedupResult {
    pub unique_artifacts: Vec<Artifact>,
    pub duplicates: Vec<DuplicateGroup>,
}

#[derive(Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub representative: Artifact,
    pub duplicates: Vec<Artifact>,
}
```

## 12. Scoring Contract

```rust
pub trait Scorer {
    async fn score(&self, artifact: &Artifact, validation: &ValidationResult) -> Result<ScoreResult, ScoreError>;
}

#[derive(Serialize, Deserialize)]
pub struct ScoreResult {
    pub grade: ScoreGrade,
    pub total_score: f64,
    pub breakdown: ScoreBreakdown,
}

#[derive(Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub metadata_confidence: f64,
    pub path_confidence: f64,
    pub integrity_confidence: f64,
    pub fragmentation_risk: f64,
    pub overwrite_risk: f64,
    pub parser_confidence: f64,
    pub validator_confidence: f64,
    pub source_reliability: f64,
}
```

## 13. Result Store Contract

```rust
pub trait ResultStore {
    async fn save_session(&self, session: &ScanSession) -> Result<i64, StoreError>;
    async fn save_artifacts(&self, artifacts: &[Artifact]) -> Result<(), StoreError>;
    async fn get_artifacts(&self, session_id: i64) -> Result<Vec<Artifact>, StoreError>;
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StoreError>;
    async fn load_checkpoint(&self, session_id: i64) -> Result<Option<Checkpoint>, StoreError>;
}

#[derive(Serialize, Deserialize)]
pub struct ScanSession {
    pub id: Option<i64>,
    pub project_id: i64,
    pub source_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
    pub engine_version: String,
    pub config: Value,
}

#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    pub session_id: i64,
    pub offset: u64,
    pub state: Value,
}
```

## 14. Job Orchestrator Contract

```rust
pub trait JobOrchestrator {
    async fn submit_job(&self, job: Job) -> Result<JobHandle, OrchestratorError>;
    async fn cancel_job(&self, handle: &JobHandle) -> Result<(), OrchestratorError>;
    fn status(&self, handle: &JobHandle) -> JobStatus;
}

#[derive(Serialize, Deserialize)]
pub struct Job {
    pub job_type: String,
    pub payload: Value,
    pub priority: JobPriority,
}

pub enum JobPriority {
    Low,
    Normal,
    High,
}

pub struct JobHandle {
    pub id: String,
}

pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}
```

## 15. UI IPC Contract

התקשורת בין Rust ל-JS דרך Tauri IPC.

### Commands from UI

```rust
#[tauri::command]
async fn create_project(name: String, description: Option<String>) -> Result<Project, String>;

#[tauri::command]
async fn list_projects() -> Result<Vec<Project>, String>;

#[tauri::command]
async fn start_scan(project_id: i64, config: ScanConfig) -> Result<String, String>; // returns job_id

#[tauri::command]
async fn get_scan_progress(job_id: String) -> Result<ScanProgress, String>;

#[tauri::command]
async fn get_artifacts(session_id: i64, filters: ArtifactFilters) -> Result<Vec<Artifact>, String>;
```

### Events to UI

```rust
// Scan progress updates
tauri::api::event::emit(&app_handle, "scan-progress", progress_data);

// New artifacts found
tauri::api::event::emit(&app_handle, "artifacts-updated", artifact_data);
```

## כללי שימוש

- כל traits חייבים להיות Send + Sync
- שגיאות חייבות להיות מסוג thiserror::Error
- כל structs חייבים להיות Serialize/Deserialize
- async functions משתמשים ב-tokio runtime
- כל I/O הוא async