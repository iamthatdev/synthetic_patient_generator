use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EvalQueryType {
    Lookup,
    FilteredLookup,
    Aggregation,
    MultiHop,
    Negative,
    Comparative,
    EvidenceRetrieval,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundTruthFact {
    pub patient_id: String,
    pub fact: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalMetadata {
    pub seed: u64,
    pub batch_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalRecord {
    pub eval_id: String,
    pub query: String,
    pub answer: String,
    pub ground_truth_patient_ids: Vec<String>,
    pub ground_truth_facts: Vec<GroundTruthFact>,
    pub contexts: Vec<String>,
    pub context_chunk_ids: Vec<String>,
    pub query_type: EvalQueryType,
    pub difficulty: Difficulty,
    pub metadata: EvalMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RagasRecord {
    pub question: String,
    pub answer: String,
    pub contexts: Vec<String>,
    pub ground_truth: String,
}
