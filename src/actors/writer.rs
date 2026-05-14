use std::path::PathBuf;

use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::output::JsonlWriter;

pub struct PatientWriterActor;

pub struct WriterState {
    patients_writer: JsonlWriter,
    notes_writer: JsonlWriter,
    chunks_writer: JsonlWriter,
    orchestrator: ActorRef<PipelineMsg>,
    total_written: usize,
}

pub struct WriterActorArgs {
    pub output_dir: PathBuf,
    pub orchestrator: ActorRef<PipelineMsg>,
}

#[async_trait::async_trait]
impl Actor for PatientWriterActor {
    type Msg = PipelineMsg;
    type State = WriterState;
    type Arguments = WriterActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tokio::fs::create_dir_all(&args.output_dir).await?;

        let patients_writer =
            JsonlWriter::create(&args.output_dir.join("patients.jsonl")).await?;
        let notes_writer =
            JsonlWriter::create(&args.output_dir.join("clinical_notes.jsonl")).await?;
        let chunks_writer = JsonlWriter::create(&args.output_dir.join("chunks.jsonl")).await?;

        Ok(WriterState {
            patients_writer,
            notes_writer,
            chunks_writer,
            orchestrator: args.orchestrator,
            total_written: 0,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::ChunkBatchCreated {
                job_id,
                batch_id,
                output,
            } => {
                let started = std::time::Instant::now();
                let count = output.len();

                for patient_output in &output {
                    state
                        .patients_writer
                        .write(&patient_output.record)
                        .await?;
                }

                for patient_output in &output {
                    for note in &patient_output.record.clinical_notes {
                        state.notes_writer.write(note).await?;
                    }
                }

                for patient_output in &output {
                    for chunk in &patient_output.chunks {
                        state.chunks_writer.write(chunk).await?;
                    }
                }

                state.total_written += count;

                tracing::info!(
                    actor = "WriterActor",
                    batch_id,
                    records = count,
                    total = state.total_written,
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Batch written"
                );

                state.orchestrator.cast(PipelineMsg::BatchWritten {
                    job_id,
                    batch_id,
                    records_written: count,
                })?;
            }
            PipelineMsg::Shutdown => {
                state.patients_writer.flush().await?;
                state.notes_writer.flush().await?;
                state.chunks_writer.flush().await?;
                tracing::info!(
                    actor = "WriterActor",
                    total = state.total_written,
                    "Writer flushed and shut down"
                );
            }
            _ => {}
        }
        Ok(())
    }
}
