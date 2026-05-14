use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalNote {
    pub note_id: String,
    pub patient_id: String,
    pub note_type: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusChunk {
    pub chunk_id: String,
    pub patient_id: String,
    pub text: String,
    pub source: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientOutput {
    pub record: super::patient::PatientRecord,
    pub chunks: Vec<CorpusChunk>,
}
