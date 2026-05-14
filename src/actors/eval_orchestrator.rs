use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::config::EvalsConfig;
use crate::domain::eval::EvalRecord;
use crate::eval_generation::EvalContext;

const EVAL_SEED_SALT: u64 = 0xDEAD_BEEF_CAFE_BABE_u64;

#[derive(Clone, Debug)]
pub enum EvalMsg {
    GenerateEvalBatch {
        job_id: String,
        batch_id: u64,
        start_index: u64,
        size: usize,
        seed: u64,
    },
    EvalBatchGenerated {
        job_id: String,
        batch_id: u64,
        evals: Vec<EvalRecord>,
    },
    EvalBatchWritten {
        job_id: String,
        batch_id: u64,
        count: usize,
    },
    EvalBatchFailed {
        job_id: String,
        batch_id: u64,
        reason: String,
    },
    Shutdown,
}

pub struct EvalOrchestratorActor;

pub struct EvalOrchestratorState {
    job_id: String,
    total_batches: usize,
    completed_batches: std::collections::HashSet<u64>,
    failed_batches: Vec<(u64, String)>,
    started_at: std::time::Instant,
    query_actor: ActorRef<EvalMsg>,
    writer_actor: ActorRef<EvalMsg>,
}

pub struct EvalOrchestratorArgs {
    pub job_id: String,
    pub eval_count: u64,
    pub batch_size: usize,
    pub seed: u64,
    pub eval_config: EvalsConfig,
    pub eval_context: EvalContext,
    pub output_dir: std::path::PathBuf,
}

#[async_trait::async_trait]
impl Actor for EvalOrchestratorActor {
    type Msg = EvalMsg;
    type State = EvalOrchestratorState;
    type Arguments = EvalOrchestratorArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let total_batches =
            ((args.eval_count as usize) + args.batch_size - 1) / args.batch_size;

        let (writer_actor, _) = EvalWriterActor::spawn(
            Some("eval_writer".to_string()),
            EvalWriterActor,
            EvalWriterArgs {
                output_dir: args.output_dir.clone(),
                orchestrator: myself.clone(),
            },
        )
        .await?;

        let (query_actor, _) = EvalQueryActor::spawn(
            Some("eval_query".to_string()),
            EvalQueryActor,
            EvalQueryArgs {
                downstream: writer_actor.clone(),
                eval_config: args.eval_config.clone(),
                eval_context: args.eval_context,
            },
        )
        .await?;

        let started_at = std::time::Instant::now();

        for batch_idx in 0..total_batches {
            let start_index = (batch_idx * args.batch_size) as u64;
            let remaining = args.eval_count - start_index;
            let batch_len = args.batch_size.min(remaining as usize);

            query_actor.cast(EvalMsg::GenerateEvalBatch {
                job_id: args.job_id.clone(),
                batch_id: batch_idx as u64,
                start_index,
                size: batch_len,
                seed: args.seed,
            })?;
        }

        tracing::info!(
            job_id = %args.job_id,
            total_batches,
            "Eval pipeline started"
        );

        Ok(EvalOrchestratorState {
            job_id: args.job_id,
            total_batches,
            completed_batches: std::collections::HashSet::new(),
            failed_batches: Vec::new(),
            started_at,
            query_actor,
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
            EvalMsg::EvalBatchWritten { batch_id, .. } => {
                state.completed_batches.insert(batch_id);

                let completed = state.completed_batches.len();
                if completed == state.total_batches {
                    let elapsed = state.started_at.elapsed();
                    tracing::info!(
                        job_id = %state.job_id,
                        evals_generated = state.total_batches * completed,
                        duration_secs = elapsed.as_secs_f64(),
                        "Eval pipeline complete"
                    );

                    state.query_actor.cast(EvalMsg::Shutdown).ok();
                    state.writer_actor.cast(EvalMsg::Shutdown).ok();
                    myself.stop(None);
                }
            }
            EvalMsg::EvalBatchFailed {
                batch_id,
                reason, ..
            } => {
                tracing::error!(batch_id, reason = %reason, "Eval batch failed");
                state.failed_batches.push((batch_id, reason));
            }
            _ => {}
        }
        Ok(())
    }
}

// --- EvalQueryActor ---

pub struct EvalQueryActor;

pub struct EvalQueryState {
    downstream: ActorRef<EvalMsg>,
    eval_config: EvalsConfig,
    eval_context: EvalContext,
}

pub struct EvalQueryArgs {
    pub downstream: ActorRef<EvalMsg>,
    pub eval_config: EvalsConfig,
    pub eval_context: EvalContext,
}

#[async_trait::async_trait]
impl Actor for EvalQueryActor {
    type Msg = EvalMsg;
    type State = EvalQueryState;
    type Arguments = EvalQueryArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(EvalQueryState {
            downstream: args.downstream,
            eval_config: args.eval_config,
            eval_context: args.eval_context,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            EvalMsg::GenerateEvalBatch {
                job_id,
                batch_id,
                start_index,
                size,
                seed,
            } => {
                let started = std::time::Instant::now();
                let mut batch_rng = crate::rng::batch_rng(seed.wrapping_add(EVAL_SEED_SALT), batch_id);

                let evals: Vec<EvalRecord> = (0..size)
                    .filter_map(|i| {
                        let eval_index = start_index + i as u64;
                        let mut prng = crate::rng::patient_rng(&mut batch_rng, i as u64);
                        crate::eval_generation::generate_eval_record(
                            &mut prng,
                            &state.eval_context,
                            eval_index,
                            &state.eval_config,
                        )
                    })
                    .collect();

                tracing::info!(
                    actor = "EvalQueryActor",
                    batch_id,
                    records = evals.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Eval batch generated"
                );

                state.downstream.cast(EvalMsg::EvalBatchGenerated {
                    job_id,
                    batch_id,
                    evals,
                })?;
            }
            EvalMsg::Shutdown => {
                state.downstream.cast(EvalMsg::Shutdown)?;
            }
            _ => {}
        }
        Ok(())
    }
}

// --- EvalWriterActor ---

pub struct EvalWriterActor;

pub struct EvalWriterState {
    evals_writer: crate::output::JsonlWriter,
    ragas_writer: crate::output::JsonlWriter,
    orchestrator: ActorRef<EvalMsg>,
    total_written: usize,
}

pub struct EvalWriterArgs {
    pub output_dir: std::path::PathBuf,
    pub orchestrator: ActorRef<EvalMsg>,
}

#[async_trait::async_trait]
impl Actor for EvalWriterActor {
    type Msg = EvalMsg;
    type State = EvalWriterState;
    type Arguments = EvalWriterArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tokio::fs::create_dir_all(&args.output_dir).await?;

        let evals_writer =
            crate::output::JsonlWriter::create(&args.output_dir.join("evals.jsonl")).await?;
        let ragas_writer =
            crate::output::JsonlWriter::create(&args.output_dir.join("ragas_dataset.jsonl")).await?;

        Ok(EvalWriterState {
            evals_writer,
            ragas_writer,
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
            EvalMsg::EvalBatchGenerated {
                job_id,
                batch_id,
                evals,
            } => {
                let started = std::time::Instant::now();
                let count = evals.len();

                for eval in &evals {
                    state.evals_writer.write(eval).await?;
                    let ragas = crate::eval_generation::to_ragas_record(eval);
                    state.ragas_writer.write(&ragas).await?;
                }

                state.total_written += count;

                tracing::info!(
                    actor = "EvalWriterActor",
                    batch_id,
                    records = count,
                    total = state.total_written,
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Eval batch written"
                );

                state.orchestrator.cast(EvalMsg::EvalBatchWritten {
                    job_id,
                    batch_id,
                    count,
                })?;
            }
            EvalMsg::Shutdown => {
                state.evals_writer.flush().await?;
                state.ragas_writer.flush().await?;
                tracing::info!(
                    actor = "EvalWriterActor",
                    total = state.total_written,
                    "Eval writer flushed and shut down"
                );
            }
            _ => {}
        }
        Ok(())
    }
}
