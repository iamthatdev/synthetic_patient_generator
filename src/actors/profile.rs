use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::config::DemographicsConfig;
use crate::generation::generate_profile;
use crate::rng;

pub struct ProfileActor;

pub struct ProfileState {
    downstream: ActorRef<PipelineMsg>,
    demographics: DemographicsConfig,
}

pub struct ProfileActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
    pub demographics: DemographicsConfig,
}

#[async_trait::async_trait]
impl Actor for ProfileActor {
    type Msg = PipelineMsg;
    type State = ProfileState;
    type Arguments = ProfileActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ProfileState {
            downstream: args.downstream,
            demographics: args.demographics,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::GeneratePatientBatch {
                job_id,
                batch_id,
                start_index,
                size,
                seed,
            } => {
                let started = std::time::Instant::now();
                let mut batch_rng = rng::batch_rng(seed, batch_id);
                let profiles: Vec<_> = (0..size)
                    .map(|i| {
                        let global_index = start_index + i as u64;
                        let mut prng = rng::patient_rng(&mut batch_rng, i as u64);
                        generate_profile(&mut prng, global_index, &state.demographics)
                    })
                    .collect();

                tracing::info!(
                    actor = "ProfileActor",
                    batch_id,
                    records = size,
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Profile batch created"
                );

                state
                    .downstream
                    .cast(PipelineMsg::ProfileBatchCreated {
                        job_id,
                        batch_id,
                        profiles,
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
