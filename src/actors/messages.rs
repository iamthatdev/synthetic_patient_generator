#[derive(Clone, Debug)]
pub enum PipelineMsg {
    GeneratePatientBatch {
        job_id: String,
        batch_id: u64,
        start_index: u64,
        size: usize,
        seed: u64,
    },
    ProfileBatchCreated {
        job_id: String,
        batch_id: u64,
        profiles: Vec<crate::domain::patient::PatientProfile>,
    },
    ConditionBatchAssigned {
        job_id: String,
        batch_id: u64,
        patients: Vec<crate::domain::patient::PatientWithConditions>,
    },
    MedicationBatchAssigned {
        job_id: String,
        batch_id: u64,
        patients: Vec<crate::domain::patient::PatientWithMedications>,
    },
    ReactionBatchSimulated {
        job_id: String,
        batch_id: u64,
        patients: Vec<crate::domain::patient::PatientRecordDraft>,
    },
    ClinicalNoteBatchGenerated {
        job_id: String,
        batch_id: u64,
        records: Vec<crate::domain::patient::PatientRecord>,
    },
    ChunkBatchCreated {
        job_id: String,
        batch_id: u64,
        output: Vec<crate::domain::clinical_note::PatientOutput>,
    },
    BatchWritten {
        job_id: String,
        batch_id: u64,
        records_written: usize,
    },
    BatchFailed {
        job_id: String,
        batch_id: u64,
        actor: String,
        reason: String,
    },
    Shutdown,
}
