use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;

// מבנה הקובץ שנרצה לשמור (תואם לנתונים שחילצנו מה-MFT)
pub struct DbArtifact {
    pub session_id: i64,
    pub mft_id: u64,
    pub parent_mft_id: u64,
    pub filename: String,
    pub is_dir: bool,
    pub is_resident: bool,
    pub real_size: u64,    // שדה שהוסר בטעות
    pub mime_type: String, // שדה חדש לזיהוי מהיר בממשק
    pub is_deleted: bool,  // האם הקובץ סומן כמחוק ב-MFT
    pub disk_offset: u64,  // המיקום הפיזי על הדיסק (לאיחזור נתונים)
}

pub struct ResultStore {
    pub conn: Connection,
}

impl ResultStore {
    // יצירת חיבור או פתיחת קובץ DB קיים
    pub fn open<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    // הקמת הסכמה הבסיסית (Tables)
    fn init_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scan_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_path TEXT NOT NULL,
                started_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS artifacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                mft_id INTEGER NOT NULL,
                parent_mft_id INTEGER NOT NULL,
                filename TEXT NOT NULL,
                is_dir BOOLEAN NOT NULL,
                is_resident BOOLEAN NOT NULL,
                real_size INTEGER NOT NULL,
                mime_type TEXT NOT NULL,
                is_deleted BOOLEAN NOT NULL,
                disk_offset INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES scan_sessions(id)
            );

            -- אינדקסים לחיפוש מהיר של עצים ותיקיות
            CREATE INDEX IF NOT EXISTS idx_parent_mft ON artifacts(session_id, parent_mft_id);
            CREATE INDEX IF NOT EXISTS idx_mft_id ON artifacts(session_id, mft_id);"
        )?;
        Ok(())
    }

    // יצירת סשן סריקה חדש
    pub fn create_session(&self, source_path: &str) -> SqlResult<i64> {
        self.conn.execute(
            "INSERT INTO scan_sessions (source_path) VALUES (?1)",
            params![source_path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    // הוספת קובץ בודד ל-DB
    pub fn insert_artifact(&self, artifact: &DbArtifact) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO artifacts (session_id, mft_id, parent_mft_id, filename, is_dir, is_resident, real_size, mime_type, is_deleted, disk_offset)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                artifact.session_id,
                artifact.mft_id,
                artifact.parent_mft_id,
                artifact.filename,
                artifact.is_dir,
                artifact.is_resident,
                artifact.real_size,
                artifact.mime_type,
                artifact.is_deleted,
                artifact.disk_offset,
            ],
        )?;
        Ok(())
    }

    // הוספת מערך שלם של קבצים במכה אחת (Bulk Insert)
    pub fn insert_artifacts_batch(&mut self, artifacts: &[DbArtifact]) -> SqlResult<()> {
        // מתחילים טרנזקציה
        let tx = self.conn.transaction()?;

        {
            // מכינים את השאילתה פעם אחת בלבד (הרבה יותר יעיל)
            let mut stmt = tx.prepare(
                "INSERT INTO artifacts (session_id, mft_id, parent_mft_id, filename, is_dir, is_resident, real_size, mime_type, is_deleted, disk_offset)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            )?;

            // רצים על כל ה-1000 ומכניסים לטרנזקציה
            for artifact in artifacts {
                stmt.execute(params![
                    artifact.session_id,
                    artifact.mft_id,
                    artifact.parent_mft_id,
                    artifact.filename,
                    artifact.is_dir,
                    artifact.is_resident,
                    artifact.real_size,
                    artifact.mime_type,
                    artifact.is_deleted,
                    artifact.disk_offset
                ])?;
            }
        } // stmt נסגרת כאן אוטומטית כי יצאנו מהבלוק

        // עכשיו שופכים הכל לדיסק בפעולה פיזית אחת!
        tx.commit()?;
        Ok(())
    }
}
