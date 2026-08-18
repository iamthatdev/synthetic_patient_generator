use std::collections::HashSet;

use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::guardrail::{GuardrailActor, GuardrailActorArgs, GuardrailConfig};
use crate::actors::messages::PipelineMsg;
use crate::actors::{
    chunking::{ChunkingActor, ChunkingActorArgs},
    condition::{ConditionActor, ConditionActorArgs},
    eval_orchestrator::{EvalOrchestratorActor, EvalOrchestratorArgs},
    medication::{MedicationActor, MedicationActorArgs},
    note::{ClinicalNoteActor, ClinicalNoteActorArgs},
    profile::{ProfileActor, ProfileActorArgs},
    reaction::{ReactionActor, ReactionActorArgs},
    writer::{PatientWriterActor, WriterActorArgs},
};
use crate::config::JobConfig;
use crate::domain::patient::PatientRecord;
use crate::eval_generation::EvalContext;

pub struct OrchestratorActor;

pub struct OrchestratorState {
    job_id: String,
    config: JobConfig,
    total_batches: usize,
    completed_batches: HashSet<u64>,
    failed_batches: Vec<(u64, String)>,
    started_at: std::time::Instant,
    eval_launched: bool,

    profile_actor: ActorRef<PipelineMsg>,
    writer_actor: ActorRef<PipelineMsg>,
}

#[async_trait::async_trait]
impl Actor for OrchestratorActor {
    type Msg = PipelineMsg;
    type State = OrchestratorState;
    type Arguments = JobConfig;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        config: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let job_id = format!(
            "job_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        tracing::info!(
            job_id = %job_id,
            patient_count = config.patient_count,
            eval_count = config.eval_count,
            batch_size = config.batch_size,
            seed = config.seed,
            "Orchestrator starting"
        );

        let (writer_actor, _) = PatientWriterActor::spawn(
            Some("patient_writer".to_string()),
            PatientWriterActor,
            WriterActorArgs {
                output_dir: config.output_dir.clone(),
                orchestrator: myself.clone(),
            },
        )
        .await?;

        let (chunking_actor, _) = ChunkingActor::spawn(
            Some("chunking".to_string()),
            ChunkingActor,
            ChunkingActorArgs {
                downstream: writer_actor.clone(),
            },
        )
        .await?;

        let guardrail_config = GuardrailConfig {
            pii_check_enabled: true,
            content_policy_enabled: true,
            plausibility_check_enabled: true,
            distribution_check_enabled: true,
            uniqueness_check_enabled: true,
            distribution_tolerance: 0.05,
            fail_on_error: false,
        };

        let (guardrail_actor, _) = GuardrailActor::spawn(
            Some("guardrail".to_string()),
            GuardrailActor,
            GuardrailActorArgs {
                downstream: chunking_actor,
                config: guardrail_config,
                conditions_config: config.conditions.clone(),
                job_id: job_id.clone(),
                output_dir: config.output_dir.clone(),
            },
        )
        .await?;

        let (note_actor, _) = ClinicalNoteActor::spawn(
            Some("clinical_note".to_string()),
            ClinicalNoteActor,
            ClinicalNoteActorArgs {
                downstream: guardrail_actor,
            },
        )
        .await?;

        let (reaction_actor, _) = ReactionActor::spawn(
            Some("reaction".to_string()),
            ReactionActor,
            ReactionActorArgs {
                downstream: note_actor,
                reactions: config.reactions.clone(),
            },
        )
        .await?;

        let (medication_actor, _) = MedicationActor::spawn(
            Some("medication".to_string()),
            MedicationActor,
            MedicationActorArgs {
                downstream: reaction_actor,
                medications: config.medications.clone(),
            },
        )
        .await?;

        let (condition_actor, _) = ConditionActor::spawn(
            Some("condition".to_string()),
            ConditionActor,
            ConditionActorArgs {
                downstream: medication_actor,
                conditions: config.conditions.clone(),
            },
        )
        .await?;

        let (profile_actor, _) = ProfileActor::spawn(
            Some("profile".to_string()),
            ProfileActor,
            ProfileActorArgs {
                downstream: condition_actor,
                demographics: config.demographics.clone(),
            },
        )
        .await?;

        let total_batches =
            ((config.patient_count as usize) + config.batch_size - 1) / config.batch_size;

        let started_at = std::time::Instant::now();

        for batch_idx in 0..total_batches {
            let start_index = (batch_idx * config.batch_size) as u64;
            let remaining = config.patient_count - start_index;
            let batch_len = config.batch_size.min(remaining as usize);

            profile_actor.cast(PipelineMsg::GeneratePatientBatch {
                job_id: job_id.clone(),
                batch_id: batch_idx as u64,
                start_index,
                size: batch_len,
                seed: config.seed,
            })?;
        }

        tracing::info!(
            job_id = %job_id,
            total_batches,
            "Patient batches dispatched"
        );

        Ok(OrchestratorState {
            job_id,
            config,
            total_batches,
            completed_batches: HashSet::new(),
            failed_batches: Vec::new(),
            started_at,
            eval_launched: false,
            profile_actor,
            writer_actor,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::BatchWritten { batch_id, .. } => {
                state.completed_batches.insert(batch_id);

                let completed = state.completed_batches.len();
                if completed % 10 == 0 || completed == state.total_batches {
                    let elapsed = state.started_at.elapsed();
                    let total_records = completed * state.config.batch_size;
                    let rate = total_records as f64 / elapsed.as_secs_f64();
                    tracing::info!(
                        job_id = %state.job_id,
                        completed_batches = completed,
                        total_batches = state.total_batches,
                        records_per_sec = rate.round() as u64,
                        "Patient pipeline progress"
                    );
                }

                if completed == state.total_batches && !state.eval_launched {
                    state.eval_launched = true;
                    // Shutdown cascades down the pipeline (profile -> ... -> writer), which
                    // lets each stage flush before the writer closes its files.
                    state.profile_actor.cast(PipelineMsg::Shutdown).ok();

                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                    let patient_path = state.config.output_dir.join("patients.jsonl");
                    match load_patient_records(&patient_path).await {
                        Ok(records) => {
                            let patient_records = records.len();

                            tracing::info!(
                                patients_loaded = patient_records,
                                "Patient data loaded for eval generation"
                            );

                            if state.config.eval_count > 0 {
                                let eval_ctx = EvalContext { records };

                                let args = EvalOrchestratorArgs {
                                    job_id: state.job_id.clone(),
                                    eval_count: state.config.eval_count,
                                    batch_size: state.config.batch_size,
                                    seed: state.config.seed,
                                    eval_config: state.config.evals.clone(),
                                    eval_context: eval_ctx,
                                    output_dir: state.config.output_dir.clone(),
                                };

                                let (_eval_orch, eval_handle) =
                                    EvalOrchestratorActor::spawn(
                                        Some("eval_orchestrator".to_string()),
                                        EvalOrchestratorActor,
                                        args,
                                    )
                                    .await?;

                                eval_handle.await?;

                                let total_elapsed = state.started_at.elapsed();
                                self.write_summary(state, total_elapsed, patient_records)
                                    .await;

                                println!(
                                    "Total: {} patients + {} evals in {:.2}s",
                                    patient_records,
                                    state.config.eval_count,
                                    total_elapsed.as_secs_f64()
                                );
                                println!("Output: {}", state.config.output_dir.display());
                            } else {
                                let elapsed = state.started_at.elapsed();
                                self.write_summary(state, elapsed, patient_records)
                                    .await;
                            }

                            myself.stop(None);
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to load patient records for eval");
                            myself.stop(None);
                        }
                    }
                }
            }
            PipelineMsg::BatchFailed {
                batch_id,
                actor,
                reason,
                ..
            } => {
                tracing::error!(
                    batch_id,
                    actor = %actor,
                    reason = %reason,
                    "Batch failed"
                );
                state.failed_batches.push((batch_id, reason));
            }
            _ => {}
        }
        Ok(())
    }
}

impl OrchestratorActor {
    async fn write_summary(
        &self,
        state: &OrchestratorState,
        elapsed: std::time::Duration,
        total_patients: usize,
    ) {
        let summary = serde_json::json!({
            "job_id": state.job_id,
            "patients_generated": total_patients,
            "evals_generated": state.config.eval_count,
            "seed": state.config.seed,
            "batch_size": state.config.batch_size,
            "duration_seconds": elapsed.as_secs_f64(),
            "records_per_second": total_patients as f64 / elapsed.as_secs_f64(),
            "failed_batches": state.failed_batches.len(),
            "output_files": [
                "patients.jsonl",
                "clinical_notes.jsonl",
                "chunks.jsonl",
                "evals.jsonl",
                "ragas_dataset.jsonl",
                "guardrail_report.json",
            ]
        });

        let summary_path = state.config.output_dir.join("summary.json");
        let content = serde_json::to_string_pretty(&summary).unwrap();
        tokio::fs::write(&summary_path, content).await.ok();
    }
}

async fn load_patient_records(path: &std::path::Path) -> std::io::Result<Vec<PatientRecord>> {
    let content = tokio::fs::read_to_string(path).await?;
    let records: Vec<PatientRecord> = content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    Ok(records)
}
