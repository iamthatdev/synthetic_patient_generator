use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::config::ConditionsConfig;
use crate::generation::assign_conditions;
use crate::rng;

pub struct ConditionActor;

pub struct ConditionState {
    downstream: ActorRef<PipelineMsg>,
    conditions: ConditionsConfig,
}

pub struct ConditionActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
    pub conditions: ConditionsConfig,
}

#[async_trait::async_trait]
impl Actor for ConditionActor {
    type Msg = PipelineMsg;
    type State = ConditionState;
    type Arguments = ConditionActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ConditionState {
            downstream: args.downstream,
            conditions: args.conditions,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::ProfileBatchCreated {
                job_id,
                batch_id,
                profiles,
            } => {
                let started = std::time::Instant::now();
                let mut batch_rng = rng::batch_rng(0, batch_id);
                let patients: Vec<_> = profiles
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut prng = rng::patient_rng(&mut batch_rng, i as u64);
                        assign_conditions(&mut prng, p, &state.conditions)
                    })
                    .collect();

                tracing::info!(
                    actor = "ConditionActor",
                    batch_id,
                    records = patients.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Condition batch assigned"
                );

                state
                    .downstream
                    .cast(PipelineMsg::ConditionBatchAssigned {
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
