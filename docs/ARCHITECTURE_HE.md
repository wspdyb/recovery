# ארכיטקטורת המערכת — פירוט טכני

גרסה: 1.0  
תאריך: 12 באפריל 2026  
שפת עבודה: עברית

## מבוא

מסמך זה מספק פירוט טכני עמוק של ארכיטקטורת המערכת, המבוסס על מסמך העבודה הראשי. הוא מתאר את השכבות, המודולים, הזרימות והחוזים הטכניים.

## ארכיטקטורת־על

המערכת מחולקת ל-12 שכבות עיקריות, כפי שמתואר במסמך הראשי. כל שכבה אחראית לתחום ספציפי ומתקשרת עם השכבות הסמוכות דרך חוזים מוגדרים.

### 1. Acquisition Layer

**טכנולוגיה**: Rust async I/O  
**אחריות**:
- גישה לקריאה בלבד למדיה
- קריאה של raw devices באמצעות `tokio::fs` או `async-std`
- קריאה של image files (DD, E01)
- retry policy עם exponential backoff
- bad sector handling עם logging
- checksum לקריאות נדרשות (CRC32)
- pause/resume עם state persistence
- checkpointing כל 1GB או לפי זמן

**מבנה נתונים**:
```rust
struct AcquisitionConfig {
    source: SourceType,
    retry_policy: RetryPolicy,
    buffer_size: usize,
    checksum_enabled: bool,
}

enum SourceType {
    RawDevice(String), // path
    ImageFile(String),  // path
}
```

### 2. Storage Abstraction Layer

**טכנולוגיה**: Rust traits  
**אחריות**:
- הפשטת מקור הנתונים ל-block interface אחיד
- block addressing: 512-byte blocks
- metadata בסיסי: size, block size, type

**חוזה**:
```rust
trait BlockReader {
    async fn read_block(&self, offset: u64, size: usize) -> Result<Vec<u8>>;
    fn total_blocks(&self) -> u64;
    fn block_size(&self) -> usize;
}
```

### 3. Disk Layout Layer

**טכנולוגיה**: Rust parsing  
**אחריות**:
- זיהוי MBR/GPT באמצעות magic bytes
- ניתוח partition entries
- חיפוש lost partitions עם heuristics
- validation למבני partition

**פורמטים נתמכים**:
- MBR (DOS partition table)
- GPT (EFI)

### 4. Filesystem Parsers Layer

**מודולים**:
- `fs-ntfs`: NTFS parser
- `fs-fat`: FAT16/32 parser
- `fs-exfat`: exFAT parser
- `fs-ext4`: ext4 parser

כל מודול מיישם trait אחיד:
```rust
trait FilesystemParser {
    fn parse(&self, reader: &dyn BlockReader) -> Result<FilesystemMetadata>;
}

struct FilesystemMetadata {
    root: DirectoryEntry,
    orphans: Vec<FileEntry>,
    deleted: Vec<FileEntry>,
}
```

### 5. Metadata Recovery Layer

**אלגוריתמים**:
- בניית עץ תיקיות מנתוני MFT (NTFS)
- שחזור orphan inodes (ext4)
- חיבור deleted entries

### 6. Carving Layer

**טכניקות**:
- Header/footer signatures
- Structure-aware carving
- Sliding window עם Boyer-Moore

**חתימות מובנות**:
- JPEG: `\xFF\xD8` ... `\xFF\xD9`
- PNG: `\x89PNG`
- PDF: `%PDF-`
- ZIP: `PK\x03\x04`

### 7. Fragment Reconstruction Layer

**היוריסטיקות**:
- Adjacency scoring לפי entropy
- Format-aware assembly
- Gap filling עם padding detection

### 8. Validation Layer

**Validators**:
- Content integrity checks
- Format validation
- Preview generation

### 9. Dedup + Fusion Layer

**אלגוריתמים**:
- Hash-based dedup (SHA256)
- Fuzzy matching
- Provenance tracking

### 10. Scoring Layer

**מודל ציון**:
- A: 90-100% confidence
- B: 80-89%
- C: 70-79%
- D: 60-69%
- F: <60%

### 11. Result Store Layer

**טכנולוגיה**: SQLite עם Diesel או Rusqlite  
**טבלאות**: כפי שמתואר ב-DB_SCHEMA_V1.sql

### 12. UI / Workflow Layer

**טכנולוגיה**: Tauri + React  
**תקשורת**: IPC עם JSON-RPC

## זרימות נתונים

1. **סריקה**: Acquisition → Storage Abstraction → Disk Layout → Parsers → Metadata Recovery → Carving → Reconstruction → Validation → Dedup/Fusion → Scoring → Result Store

2. **הצגה**: Result Store → UI (tree view, raw view, preview)

3. **ייצוא**: Result Store → Export (files, reports, snapshots)

## ביצועים ותזמור

- **Threading**: async/await עם tokio
- **Memory**: bounded channels ל-backpressure
- **I/O**: parallel reads עם semaphore
- **Persistence**: checkpoint כל 10 דקות או 1GB

## אבטחה

- Read-only enforcement
- Sandboxed previews
- Audit logging

## הרחבות עתידיות

- Plugin system עם WebAssembly
- Distributed scanning עם gRPC
- AI-assisted reconstruction