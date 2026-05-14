use ractor::{Actor, ActorRef, ActorProcessingErr};

use crate::actors::messages::PipelineMsg;
use crate::domain::clinical_note::ClinicalNote;
use crate::domain::patient::{PatientMetadata, PatientRecord};
use crate::generation::generate_clinical_note_text;

pub struct ClinicalNoteActor;

pub struct ClinicalNoteState {
    downstream: ActorRef<PipelineMsg>,
}

pub struct ClinicalNoteActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
}

#[async_trait::async_trait]
impl Actor for ClinicalNoteActor {
    type Msg = PipelineMsg;
    type State = ClinicalNoteState;
    type Arguments = ClinicalNoteActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(ClinicalNoteState {
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
            PipelineMsg::ReactionBatchSimulated {
                job_id,
                batch_id,
                patients,
            } => {
                let started = std::time::Instant::now();
                let seed_val = 0u64;

                let records: Vec<PatientRecord> = patients
                    .into_iter()
                    .map(|draft| {
                        let note_text = generate_clinical_note_text(&draft);
                        let note_id = format!("note_{}_0", draft.profile.patient_id);
                        let note = ClinicalNote {
                            note_id,
                            patient_id: draft.profile.patient_id.clone(),
                            note_type: "primary_care".to_string(),
                            text: note_text,
                        };
                        PatientRecord {
                            patient_id: draft.profile.patient_id,
                            name: draft.profile.name,
                            age: draft.profile.age,
                            gender: draft.profile.gender,
                            region: draft.profile.region,
                            comorbidities: draft.comorbidities,
                            medications: draft.medications,
                            allergic_reaction: draft.allergic_reaction,
                            reaction_medication: draft.reaction_medication,
                            reaction_type: draft.reaction_type,
                            reaction_severity: draft.reaction_severity,
                            clinical_notes: vec![note],
                            metadata: PatientMetadata {
                                seed: seed_val,
                                batch_id,
                            },
                        }
                    })
                    .collect();

                tracing::info!(
                    actor = "ClinicalNoteActor",
                    batch_id,
                    records = records.len(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "Clinical note batch generated"
                );

                _state
                    .downstream
                    .cast(PipelineMsg::ClinicalNoteBatchGenerated {
                        job_id,
                        batch_id,
                        records,
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
