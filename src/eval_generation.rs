use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::EvalsConfig;
use crate::domain::eval::{Difficulty, EvalQueryType, EvalRecord, GroundTruthFact, RagasRecord};
use crate::domain::patient::{Gender, PatientRecord, Severity};

pub struct EvalContext {
    pub records: Vec<PatientRecord>,
}

impl EvalContext {
    pub fn drug_x_reactors(&self) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.allergic_reaction && r.reaction_medication.as_deref() == Some("DrugX"))
            .collect()
    }

    pub fn drug_y_reactors(&self) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.allergic_reaction && r.reaction_medication.as_deref() == Some("DrugY"))
            .collect()
    }

    pub fn drug_x_exposed(&self) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.medications.contains(&"DrugX".to_string()))
            .collect()
    }

    pub fn drug_y_exposed(&self) -> Vec<&PatientRecord> {
        self.records
            .iter()
            .filter(|r| r.medications.contains(&"DrugY".to_string()))
            .collect()
    }

    pub fn find_by_id(&self, id: &str) -> Option<&PatientRecord> {
        self.records.iter().find(|r| r.patient_id == id)
    }
}

pub fn generate_eval_record(
    rng: &mut ChaCha8Rng,
    ctx: &EvalContext,
    eval_index: u64,
    config: &EvalsConfig,
) -> Option<EvalRecord> {
    let query_type = pick_query_type(rng, config);
    match query_type {
        EvalQueryType::Lookup => generate_lookup(rng, ctx, eval_index),
        EvalQueryType::FilteredLookup => generate_filtered_lookup(rng, ctx, eval_index),
        EvalQueryType::Aggregation => generate_aggregation(rng, ctx, eval_index),
        EvalQueryType::MultiHop => generate_multihop(rng, ctx, eval_index),
        EvalQueryType::Negative => generate_negative(rng, ctx, eval_index),
        EvalQueryType::Comparative => generate_comparative(rng, ctx, eval_index),
        EvalQueryType::EvidenceRetrieval => generate_evidence(rng, ctx, eval_index),
    }
}

fn pick_query_type(rng: &mut ChaCha8Rng, config: &EvalsConfig) -> EvalQueryType {
    let roll: f64 = rng.gen();
    if config.include_aggregation_queries && roll < 0.15 {
        EvalQueryType::Aggregation
    } else if config.include_multihop_queries && roll < 0.30 {
        EvalQueryType::MultiHop
    } else if config.include_negative_controls && roll < 0.40 {
        EvalQueryType::Negative
    } else if roll < 0.55 {
        EvalQueryType::Comparative
    } else if roll < 0.75 {
        EvalQueryType::EvidenceRetrieval
    } else if roll < 0.88 {
        EvalQueryType::FilteredLookup
    } else {
        EvalQueryType::Lookup
    }
}

fn generate_lookup(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let med = if rng.gen::<f64>() < 0.6 { "DrugX" } else { "DrugY" };
    let matching: Vec<_> = ctx.records.iter()
        .filter(|r| r.allergic_reaction && r.reaction_medication.as_deref() == Some(med))
        .collect();

    if matching.is_empty() {
        return generate_filtered_lookup(rng, ctx, idx);
    }

    let patient_ids: Vec<String> = matching.iter().map(|r| r.patient_id.clone()).collect();
    let answer = patient_ids.join(", ");
    let contexts: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone()))
        .collect();
    let chunk_ids: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().enumerate().map(|(i, _)| format!("chunk_{}_note_{}", r.patient_id, i)))
        .collect();

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("Which patients had a reaction to {}?", med),
        answer,
        ground_truth_patient_ids: patient_ids,
        ground_truth_facts: matching.iter().map(|r| GroundTruthFact {
            patient_id: r.patient_id.clone(),
            fact: format!("medication {}, reaction {}", med, r.reaction_type.as_deref().unwrap_or("unknown")),
        }).collect(),
        contexts,
        context_chunk_ids: chunk_ids,
        query_type: EvalQueryType::Lookup,
        difficulty: Difficulty::Easy,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_filtered_lookup(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let min_age: u8 = rng.gen_range(50..=75);
    let gender = if rng.gen::<f64>() < 0.5 { Gender::Female } else { Gender::Male };
    let gender_str = match gender { Gender::Female => "female", Gender::Male => "male" };
    let med = "DrugX";

    let matching: Vec<_> = ctx.records.iter()
        .filter(|r| {
            r.allergic_reaction
            && r.reaction_medication.as_deref() == Some(med)
            && r.age > min_age
            && r.gender == gender
        })
        .collect();

    let patient_ids: Vec<String> = matching.iter().map(|r| r.patient_id.clone()).collect();
    let answer = if patient_ids.is_empty() { "None".to_string() } else { patient_ids.join(", ") };
    let contexts: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone()))
        .collect();
    let chunk_ids: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().enumerate().map(|(i, _)| format!("chunk_{}_note_{}", r.patient_id, i)))
        .collect();

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("Which {} patients over {} had a reaction to {}?", gender_str, min_age, med),
        answer,
        ground_truth_patient_ids: patient_ids,
        ground_truth_facts: matching.iter().map(|r| GroundTruthFact {
            patient_id: r.patient_id.clone(),
            fact: format!("age > {}, gender {}, medication {}, reaction {}", min_age, gender_str, med, r.reaction_type.as_deref().unwrap_or("unknown")),
        }).collect(),
        contexts,
        context_chunk_ids: chunk_ids,
        query_type: EvalQueryType::FilteredLookup,
        difficulty: Difficulty::Medium,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_aggregation(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let med = if rng.gen::<f64>() < 0.6 { "DrugX" } else { "DrugY" };
    let exposed = if med == "DrugX" { ctx.drug_x_exposed() } else { ctx.drug_y_exposed() };
    let reactors: Vec<_> = exposed.iter()
        .filter(|r| r.allergic_reaction && r.reaction_medication.as_deref() == Some(med))
        .collect();
    let severe: Vec<_> = reactors.iter()
        .filter(|r| matches!(r.reaction_severity, Some(Severity::Severe)))
        .collect();

    let pct = if exposed.is_empty() { 0.0 } else { (reactors.len() as f64 / exposed.len() as f64) * 100.0 };
    let answer = format!("{:.1}% ({} out of {} exposed)", pct, reactors.len(), exposed.len());

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("What percentage of {} patients had an adverse reaction?", med),
        answer,
        ground_truth_patient_ids: reactors.iter().map(|r| r.patient_id.clone()).collect(),
        ground_truth_facts: vec![GroundTruthFact {
            patient_id: "_aggregate".to_string(),
            fact: format!("{} exposed: {}, reactors: {}, severe: {}", med, exposed.len(), reactors.len(), severe.len()),
        }],
        contexts: reactors.iter().take(5).flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone())).collect(),
        context_chunk_ids: reactors.iter().take(5).flat_map(|r| r.clinical_notes.iter().enumerate().map(|(i, _)| format!("chunk_{}_note_{}", r.patient_id, i))).collect(),
        query_type: EvalQueryType::Aggregation,
        difficulty: Difficulty::Medium,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_multihop(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let condition = if rng.gen::<f64>() < 0.5 { "diabetes" } else { "hypertension" };
    let med = "DrugX";

    let matching: Vec<_> = ctx.records.iter()
        .filter(|r| {
            r.comorbidities.iter().any(|c| c.to_lowercase() == condition)
            && r.medications.contains(&med.to_string())
            && r.allergic_reaction
            && r.reaction_medication.as_deref() == Some(med)
        })
        .collect();

    let patient_ids: Vec<String> = matching.iter().map(|r| r.patient_id.clone()).collect();
    let answer = if patient_ids.is_empty() { "None".to_string() } else { format!("{}", patient_ids.len()) };
    let contexts: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone()))
        .collect();
    let chunk_ids: Vec<String> = matching.iter()
        .flat_map(|r| r.clinical_notes.iter().enumerate().map(|(i, _)| format!("chunk_{}_note_{}", r.patient_id, i)))
        .collect();

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("Among {} patients prescribed {}, how many had an adverse reaction?", condition, med),
        answer,
        ground_truth_patient_ids: patient_ids,
        ground_truth_facts: matching.iter().map(|r| GroundTruthFact {
            patient_id: r.patient_id.clone(),
            fact: format!("{}, {}, reaction to {}", condition, med, r.reaction_type.as_deref().unwrap_or("unknown")),
        }).collect(),
        contexts,
        context_chunk_ids: chunk_ids,
        query_type: EvalQueryType::MultiHop,
        difficulty: Difficulty::Hard,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_negative(_rng: &mut ChaCha8Rng, _ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let fake_med = "DrugQ";

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("Which patients had an allergy to {}?", fake_med),
        answer: "None".to_string(),
        ground_truth_patient_ids: vec![],
        ground_truth_facts: vec![],
        contexts: vec![],
        context_chunk_ids: vec![],
        query_type: EvalQueryType::Negative,
        difficulty: Difficulty::Easy,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_comparative(_rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let x_reactors = ctx.drug_x_reactors();
    let y_reactors = ctx.drug_y_reactors();

    let answer = if x_reactors.len() > y_reactors.len() {
        format!("DrugX ({} reactions vs {} for DrugY)", x_reactors.len(), y_reactors.len())
    } else if y_reactors.len() > x_reactors.len() {
        format!("DrugY ({} reactions vs {} for DrugX)", y_reactors.len(), x_reactors.len())
    } else {
        format!("Equal ({} reactions each)", x_reactors.len())
    };

    let all_reactors: Vec<_> = x_reactors.iter().chain(y_reactors.iter()).collect();
    let contexts: Vec<String> = all_reactors.iter().take(5)
        .flat_map(|r| r.clinical_notes.iter().map(|n| n.text.clone()))
        .collect();
    let chunk_ids: Vec<String> = all_reactors.iter().take(5)
        .flat_map(|r| r.clinical_notes.iter().enumerate().map(|(i, _)| format!("chunk_{}_note_{}", r.patient_id, i)))
        .collect();

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: "Was DrugX or DrugY associated with more adverse reactions?".to_string(),
        answer,
        ground_truth_patient_ids: all_reactors.iter().map(|r| r.patient_id.clone()).collect(),
        ground_truth_facts: vec![
            GroundTruthFact { patient_id: "_comparative".to_string(), fact: format!("DrugX reactions: {}", x_reactors.len()) },
            GroundTruthFact { patient_id: "_comparative".to_string(), fact: format!("DrugY reactions: {}", y_reactors.len()) },
        ],
        contexts,
        context_chunk_ids: chunk_ids,
        query_type: EvalQueryType::Comparative,
        difficulty: Difficulty::Medium,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

fn generate_evidence(rng: &mut ChaCha8Rng, ctx: &EvalContext, idx: u64) -> Option<EvalRecord> {
    let reactors = ctx.drug_x_reactors();
    if reactors.is_empty() {
        return generate_lookup(rng, ctx, idx);
    }

    let patient = &reactors[rng.gen_range(0..reactors.len())];
    let note_text = patient.clinical_notes.first().map(|n| n.text.clone()).unwrap_or_default();

    Some(EvalRecord {
        eval_id: format!("E{:09}", idx),
        query: format!("Find clinical notes supporting DrugX allergy in patient {}.", patient.patient_id),
        answer: note_text.clone(),
        ground_truth_patient_ids: vec![patient.patient_id.clone()],
        ground_truth_facts: vec![GroundTruthFact {
            patient_id: patient.patient_id.clone(),
            fact: format!("DrugX allergy: {}", patient.reaction_type.as_deref().unwrap_or("unknown")),
        }],
        contexts: vec![note_text],
        context_chunk_ids: patient.clinical_notes.iter().enumerate()
            .map(|(i, _)| format!("chunk_{}_note_{}", patient.patient_id, i))
            .collect(),
        query_type: EvalQueryType::EvidenceRetrieval,
        difficulty: Difficulty::Easy,
        metadata: crate::domain::eval::EvalMetadata { seed: 0, batch_id: 0 },
    })
}

pub fn to_ragas_record(eval: &EvalRecord) -> RagasRecord {
    RagasRecord {
        question: eval.query.clone(),
        answer: eval.answer.clone(),
        contexts: eval.contexts.clone(),
        ground_truth: eval.ground_truth_patient_ids.join(", "),
    }
}
