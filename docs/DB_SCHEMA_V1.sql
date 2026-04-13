-- PhoenixRecover Database Schema v1
-- גרסה: 1.0
-- תאריך: 12 באפריל 2026
-- שפת עבודה: SQL

-- יצירת מסד הנתונים
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    version TEXT NOT NULL DEFAULT '1.0'
);

CREATE TABLE scan_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    source_type TEXT NOT NULL, -- 'device', 'image', 'partition'
    source_path TEXT NOT NULL,
    size_bytes INTEGER,
    block_size INTEGER DEFAULT 512,
    checksum TEXT, -- SHA256 של המקור
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE scan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    source_id INTEGER NOT NULL,
    started_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    ended_at DATETIME,
    status TEXT NOT NULL DEFAULT 'running', -- 'running', 'completed', 'paused', 'failed'
    engine_version TEXT NOT NULL,
    config_json TEXT, -- JSON עם הגדרות הסריקה
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (source_id) REFERENCES scan_sources(id) ON DELETE CASCADE
);

CREATE TABLE scan_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    config_json TEXT NOT NULL, -- JSON עם פרופיל הסריקה
    is_default BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE scan_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    job_type TEXT NOT NULL, -- 'metadata', 'carving', 'validation', etc.
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed'
    progress REAL DEFAULT 0.0, -- 0.0 to 1.0
    started_at DATETIME,
    ended_at DATETIME,
    error_message TEXT,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE TABLE scan_checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    job_id INTEGER,
    offset_bytes INTEGER NOT NULL,
    data_json TEXT, -- JSON עם state
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (job_id) REFERENCES scan_jobs(id) ON DELETE CASCADE
);

CREATE TABLE scan_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    event_type TEXT NOT NULL, -- 'info', 'warning', 'error'
    message TEXT NOT NULL,
    details_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE TABLE artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    artifact_type TEXT NOT NULL, -- 'file', 'directory', 'fragment'
    size_bytes INTEGER,
    offset_start INTEGER,
    offset_end INTEGER,
    confidence REAL DEFAULT 0.0, -- 0.0 to 1.0
    status TEXT NOT NULL DEFAULT 'raw', -- 'raw', 'validated', 'merged'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE TABLE artifact_names (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    name_normalized TEXT, -- lowercase, normalized
    confidence REAL DEFAULT 1.0,
    source TEXT NOT NULL, -- 'metadata', 'carving', 'reconstructed'
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE artifact_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    path_normalized TEXT, -- lowercase, normalized
    confidence REAL DEFAULT 1.0,
    source TEXT NOT NULL,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE artifact_fragments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    offset_start INTEGER NOT NULL,
    offset_end INTEGER NOT NULL,
    data_hash TEXT, -- SHA256
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE artifact_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    source_type TEXT NOT NULL, -- 'metadata_recovery', 'carver', 'reconstructor'
    source_details_json TEXT,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE artifact_hashes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    hash_type TEXT NOT NULL, -- 'sha256', 'md5'
    hash_value TEXT NOT NULL,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE validation_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    validator_type TEXT NOT NULL, -- 'jpeg', 'pdf', 'text'
    status TEXT NOT NULL, -- 'valid', 'invalid', 'partial'
    notes TEXT,
    repair_hints TEXT,
    preview_metadata_json TEXT,
    validated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE scoring_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    overall_score TEXT NOT NULL, -- 'A', 'B', 'C', 'D', 'F'
    metadata_confidence REAL,
    path_confidence REAL,
    integrity_confidence REAL,
    fragmentation_risk REAL,
    overwrite_risk REAL,
    parser_confidence REAL,
    validator_confidence REAL,
    source_reliability REAL,
    details_json TEXT,
    scored_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE merge_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    representative_artifact_id INTEGER NOT NULL,
    merged_artifact_ids TEXT NOT NULL, -- JSON array
    merge_reason TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (representative_artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    export_type TEXT NOT NULL, -- 'files', 'report', 'snapshot'
    export_path TEXT NOT NULL,
    config_json TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

CREATE TABLE preview_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_id INTEGER NOT NULL,
    preview_type TEXT NOT NULL, -- 'thumbnail', 'text', 'hex'
    preview_data BLOB,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE CASCADE
);

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER,
    user_action TEXT NOT NULL,
    details_json TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- אינדקסים
CREATE INDEX idx_scan_sessions_project ON scan_sessions(project_id);
CREATE INDEX idx_scan_sessions_started ON scan_sessions(started_at);
CREATE INDEX idx_artifacts_session ON artifacts(session_id);
CREATE INDEX idx_artifacts_size ON artifacts(size_bytes);
CREATE INDEX idx_artifact_names_normalized ON artifact_names(name_normalized);
CREATE INDEX idx_artifact_paths_normalized ON artifact_paths(path_normalized);
CREATE INDEX idx_artifact_hashes_value ON artifact_hashes(hash_value);
CREATE INDEX idx_validation_results_artifact ON validation_results(artifact_id);
CREATE INDEX idx_scoring_results_artifact ON scoring_results(artifact_id);

-- FTS5 לטבלאות חיפוש
CREATE VIRTUAL TABLE artifact_names_fts USING fts5(name, content=artifact_names, content_rowid=id);
CREATE VIRTUAL TABLE artifact_paths_fts USING fts5(path, content=artifact_paths, content_rowid=id);

-- טריגרים לעדכון FTS
CREATE TRIGGER artifact_names_fts_insert AFTER INSERT ON artifact_names
BEGIN
    INSERT INTO artifact_names_fts(rowid, name) VALUES (new.id, new.name);
END;

CREATE TRIGGER artifact_names_fts_delete AFTER DELETE ON artifact_names
BEGIN
    DELETE FROM artifact_names_fts WHERE rowid = old.id;
END;

CREATE TRIGGER artifact_names_fts_update AFTER UPDATE ON artifact_names
BEGIN
    UPDATE artifact_names_fts SET name = new.name WHERE rowid = new.id;
END;

CREATE TRIGGER artifact_paths_fts_insert AFTER INSERT ON artifact_paths
BEGIN
    INSERT INTO artifact_paths_fts(rowid, path) VALUES (new.id, new.path);
END;

CREATE TRIGGER artifact_paths_fts_delete AFTER DELETE ON artifact_paths
BEGIN
    DELETE FROM artifact_paths_fts WHERE rowid = old.id;
END;

CREATE TRIGGER artifact_paths_fts_update AFTER UPDATE ON artifact_paths
BEGIN
    UPDATE artifact_paths_fts SET path = new.path WHERE rowid = new.id;
END;