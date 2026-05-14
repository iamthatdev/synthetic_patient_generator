use std::collections::HashMap;

use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::domain::clinical_note::{CorpusChunk, PatientOutput};

pub struct ChunkingActor;

pub struct ChunkingState {
    downstream: ActorRef<PipelineMsg>,
}

pub struct ChunkingActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
}

#[async_trait::async_trait]
impl Actor for ChunkingActor {
    type Msg = PipelineMsg;
    type State = ChunkingState;
    type Arguments = ChunkingActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ChunkingState {
            downstream: args.downstream,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::ClinicalNoteBatchGenerated {
                job_id,
                batch_id,
                records,
            } => {
                let started = std::time::Instant::now();
                let output: Vec<PatientOutput> = records
                    .into_iter()
                    .map(|record| {
                        let chunks = record
                            .clinical_notes
                            .iter()
                            .enumerate()
                            .map(|(note_idx, note)| {
                                let chunk_id =
                                    format!("chunk_{}_note_{}", record.patient_id, note_idx);
                                let mut metadata = HashMap::new();
                                metadata.insert("note_type".to_string(), note.note_type.clone());
                                CorpusChunk {
                                    chunk_id,
                                    patient_id: record.patient_id.clone(),
                                    text: note.text.clone(),
                                    source: format!(
                                        "clinical_note_{}_{}",
                                        record.patient_id, note_idx
                                    ),
                                    metadata,
                                }
                            })
                            .collect();
                        PatientOutput { record, chunks }
                    })
                    .collect();

                tracing::info!(
                    actor = "ChunkingActor",
                    batch_id,
                    records = output.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Chunk batch created"
                );

                _state.downstream.cast(PipelineMsg::ChunkBatchCreated {
                    job_id,
                    batch_id,
                    output,
                })?;
            }
            PipelineMsg::Shutdown => {
                _state.downstream.cast(PipelineMsg::Shutdown)?;
            }
            _ => {}
        }
        Ok(())
    }
}
