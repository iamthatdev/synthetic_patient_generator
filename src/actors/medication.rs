use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::config::MedicationsConfig;
use crate::generation::assign_medications;
use crate::rng;

pub struct MedicationActor;

pub struct MedicationState {
    downstream: ActorRef<PipelineMsg>,
    medications: MedicationsConfig,
}

pub struct MedicationActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
    pub medications: MedicationsConfig,
}

#[async_trait::async_trait]
impl Actor for MedicationActor {
    type Msg = PipelineMsg;
    type State = MedicationState;
    type Arguments = MedicationActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(MedicationState {
            downstream: args.downstream,
            medications: args.medications,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::ConditionBatchAssigned {
                job_id,
                batch_id,
                patients,
            } => {
                let started = std::time::Instant::now();
                let mut batch_rng = rng::batch_rng(1, batch_id);
                let patients: Vec<_> = patients
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let mut prng = rng::patient_rng(&mut batch_rng, i as u64);
                        assign_medications(&mut prng, p, &state.medications)
                    })
                    .collect();

                tracing::info!(
                    actor = "MedicationActor",
                    batch_id,
                    records = patients.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Medication batch assigned"
                );

                state
                    .downstream
                    .cast(PipelineMsg::MedicationBatchAssigned {
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
