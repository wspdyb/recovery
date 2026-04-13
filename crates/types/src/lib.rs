use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Core types for PhoenixRecover

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    RawDevice(String), // path to device
    ImageFile(String),  // path to image
    Partition { device: String, start: u64, size: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    File,
    Directory,
    Fragment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoreGrade {
    A, B, C, D, F,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Confidence(f64);

impl Confidence {
    pub fn new(val: f64) -> Self {
        Self(val.clamp(0.0, 1.0))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

impl From<f64> for Confidence {
    fn from(val: f64) -> Self {
        Self::new(val)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: Option<i64>,
    pub project_id: i64,
    pub source_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
    pub engine_version: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Option<i64>,
    pub session_id: i64,
    pub artifact_type: ArtifactType,
    pub size_bytes: Option<i64>,
    pub offset_start: Option<i64>,
    pub offset_end: Option<i64>,
    pub confidence: Confidence,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub id: Option<i64>,
    pub artifact_id: i64,
    pub validator_type: String,
    pub status: ValidationStatus,
    pub notes: Option<String>,
    pub repair_hints: Option<String>,
    pub preview_metadata: Option<serde_json::Value>,
    pub validated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResult {
    pub id: Option<i64>,
    pub artifact_id: i64,
    pub grade: ScoreGrade,
    pub total_score: Confidence,
    pub breakdown: ScoreBreakdown,
    pub scored_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub metadata_confidence: Confidence,
    pub path_confidence: Confidence,
    pub integrity_confidence: Confidence,
    pub fragmentation_risk: Confidence,
    pub overwrite_risk: Confidence,
    pub parser_confidence: Confidence,
    pub validator_confidence: Confidence,
    pub source_reliability: Confidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarveSignature {
    pub name: String,
    pub header: Vec<u8>,
    pub footer: Option<Vec<u8>>,
    pub max_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Option<i64>,
    pub session_id: i64,
    pub job_id: Option<i64>,
    pub offset_bytes: i64,
    pub data_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}