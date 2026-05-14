use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::config::ReactionsConfig;
use crate::generation::simulate_reaction;
use crate::rng;

pub struct ReactionActor;

pub struct ReactionState {
    downstream: ActorRef<PipelineMsg>,
    reactions: ReactionsConfig,
}

pub struct ReactionActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
    pub reactions: ReactionsConfig,
}

#[async_trait::async_trait]
impl Actor for ReactionActor {
    type Msg = PipelineMsg;
    type State = ReactionState;
    type Arguments = ReactionActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ReactionState {
            downstream: args.downstream,
            reactions: args.reactions,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::MedicationBatchAssigned {
                job_id,
                batch_id,
                patients,
            } => {
                let started = std::time::Instant::now();
                let mut batch_rng = rng::batch_rng(2, batch_id);
                let patients: Vec<_> = patients
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut prng = rng::patient_rng(&mut batch_rng, i as u64);
                        simulate_reaction(&mut prng, p, &state.reactions)
                    })
                    .collect();

                tracing::info!(
                    actor = "ReactionActor",
                    batch_id,
                    records = patients.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Reaction batch simulated"
                );

                state
                    .downstream
                    .cast(PipelineMsg::ReactionBatchSimulated {
                        job_id,
                        batch_id,
                        patients,
                    })?;
            }
            PipelineMsg::Shutdown => {
                state.downstream.cast(PipelineMsg::Shutdown)?;
            }
            _ => {}
        }
        Ok(())
    }
}
