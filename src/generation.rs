use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::{ConditionsConfig, DemographicsConfig, MedicationsConfig, ReactionsConfig};
use crate::domain::patient::{
    Gender, PatientProfile, PatientRecordDraft, PatientWithConditions, PatientWithMedications,
    RiskBucket, Severity,
};

pub fn generate_profile(
    rng: &mut ChaCha8Rng,
    global_index: u64,
    demographics: &DemographicsConfig,
) -> PatientProfile {
    let patient_id = format!("P{:09}", global_index);
    let age: u8 = rng.gen_range(demographics.min_age..=demographics.max_age);
    let gender = if rng.gen::<f64>() < demographics.female_probability {
        Gender::Female
    } else {
        Gender::Male
    };
    let region = generate_region(rng);
    let risk_bucket = generate_risk_bucket(rng, age);

    let name = crate::data::names::generate_name(rng, &gender, &region);

    PatientProfile {
        patient_id,
        name,
        age,
        gender,
        region,
        risk_bucket,
    }
}

pub fn assign_conditions(
    rng: &mut ChaCha8Rng,
    profile: PatientProfile,
    config: &ConditionsConfig,
) -> PatientWithConditions {
    let mut comorbidities = Vec::new();
    if rng.gen::<f64>() < config.diabetes {
        comorbidities.push("diabetes".to_string());
    }
    if rng.gen::<f64>() < config.hypertension {
        comorbidities.push("hypertension".to_string());
    }
    if rng.gen::<f64>() < config.asthma {
        comorbidities.push("asthma".to_string());
    }
    if rng.gen::<f64>() < config.chronic_kidney_disease {
        comorbidities.push("chronic_kidney_disease".to_string());
    }
    if rng.gen::<f64>() < config.coronary_artery_disease {
        comorbidities.push("coronary_artery_disease".to_string());
    }
    if rng.gen::<f64>() < config.copd {
        comorbidities.push("copd".to_string());
    }
    if rng.gen::<f64>() < config.obesity {
        comorbidities.push("obesity".to_string());
    }
    PatientWithConditions {
        profile,
        comorbidities,
    }
}

pub fn assign_medications(
    rng: &mut ChaCha8Rng,
    patient: PatientWithConditions,
    config: &MedicationsConfig,
) -> PatientWithMedications {
    let mut medications = Vec::new();

    if patient.comorbidities.contains(&"diabetes".to_string())
        && rng.gen::<f64>() < config.metformin_exposure_if_diabetes
    {
        medications.push("Metformin".to_string());
    }
    if patient.comorbidities.contains(&"hypertension".to_string()) {
        medications.push("Lisinopril".to_string());
    }
    if patient.comorbidities.contains(&"asthma".to_string()) {
        medications.push("Albuterol".to_string());
    }
    if patient.comorbidities.contains(&"coronary_artery_disease".to_string())
        || (patient.profile.age > 55 && rng.gen::<f64>() < config.aspirin_exposure)
    {
        if !medications.contains(&"Aspirin".to_string()) {
            medications.push("Aspirin".to_string());
        }
    } else if rng.gen::<f64>() < config.aspirin_exposure {
        if !medications.contains(&"Aspirin".to_string()) {
            medications.push("Aspirin".to_string());
        }
    }
    if rng.gen::<f64>() < config.drug_x_exposure {
        medications.push("DrugX".to_string());
    }
    if rng.gen::<f64>() < config.drug_y_exposure {
        medications.push("DrugY".to_string());
    }
    if rng.gen::<f64>() < config.drug_z_exposure {
        medications.push("DrugZ".to_string());
    }

    PatientWithMedications {
        profile: patient.profile,
        comorbidities: patient.comorbidities,
        medications,
    }
}

pub fn simulate_reaction(
    rng: &mut ChaCha8Rng,
    patient: PatientWithMedications,
    reactions: &ReactionsConfig,
) -> PatientRecordDraft {
    let mut allergic_reaction = false;
    let mut reaction_medication = None;
    let mut reaction_type = None;
    let mut reaction_severity = None;

    let age = patient.profile.age;
    let has_diabetes = patient.comorbidities.contains(&"diabetes".to_string());

    if patient.medications.contains(&"DrugX".to_string()) {
        let mut prob = reactions.drug_x.reaction_probability;
        if has_diabetes {
            prob += 0.05;
        }
        if rng.gen::<f64>() < prob {
            allergic_reaction = true;
            reaction_medication = Some("DrugX".to_string());
            reaction_type = Some(pick_reaction_type(rng, &reactions.drug_x.reaction_types));
            let mut severe_prob = reactions.drug_x.severe_probability;
            if age > 65 {
                severe_prob += 0.10;
            }
            reaction_severity = Some(pick_severity(rng, severe_prob));
        }
    }

    if !allergic_reaction && patient.medications.contains(&"DrugY".to_string()) {
        let mut prob = reactions.drug_y.reaction_probability;
        if has_diabetes {
            prob += 0.03;
        }
        if rng.gen::<f64>() < prob {
            allergic_reaction = true;
            reaction_medication = Some("DrugY".to_string());
            reaction_type = Some(pick_reaction_type(rng, &reactions.drug_y.reaction_types));
            let mut severe_prob = reactions.drug_y.severe_probability;
            if age > 65 {
                severe_prob += 0.05;
            }
            reaction_severity = Some(pick_severity(rng, severe_prob));
        }
    }

    PatientRecordDraft {
        profile: patient.profile,
        comorbidities: patient.comorbidities,
        medications: patient.medications,
        allergic_reaction,
        reaction_medication,
        reaction_type,
        reaction_severity,
    }
}

pub fn generate_clinical_note_text(draft: &PatientRecordDraft) -> String {
    let gender_str = match draft.profile.gender {
        Gender::Female => "female",
        Gender::Male => "male",
    };
    let conditions_str = if draft.comorbidities.is_empty() {
        "no significant comorbidities".to_string()
    } else {
        draft.comorbidities.join(" and ")
    };
    let meds_str = if draft.medications.is_empty() {
        "no current medications".to_string()
    } else {
        draft.medications.join(", ")
    };

    let patient_identifier = draft.profile.name.full();

    let mut note = format!(
        "Patient {} is a {}-year-old {} with {}. Prescribed medications: {}.",
        patient_identifier, draft.profile.age, gender_str, conditions_str, meds_str,
    );

    if draft.allergic_reaction {
        if let Some(ref med) = draft.reaction_medication {
            let severity_str = match draft.reaction_severity {
                Some(Severity::Severe) => "severe",
                Some(Severity::Moderate) => "moderate",
                Some(Severity::Mild) => "mild",
                None => "reported",
            };
            let rtype = draft.reaction_type.as_deref().unwrap_or("unknown reaction");
            note.push_str(&format!(
                " Within 24 hours of {} exposure, patient developed a {} {}. {} was discontinued.",
                med, severity_str, rtype, med,
            ));
            if matches!(draft.reaction_severity, Some(Severity::Severe)) {
                note.push_str(
                    " Reaction resolved after antihistamine treatment and monitoring.",
                );
            }
        }
    }

    note
}

fn generate_region<R: Rng>(rng: &mut R) -> String {
    let regions = [
        "Northeast",
        "Southeast",
        "Midwest",
        "Southwest",
        "West",
    ];
    regions[rng.gen_range(0..regions.len())].to_string()
}

fn generate_risk_bucket<R: Rng>(rng: &mut R, age: u8) -> RiskBucket {
    let roll: f64 = rng.gen();
    if age > 70 || roll < 0.1 {
        RiskBucket::High
    } else if age > 50 || roll < 0.3 {
        RiskBucket::Medium
    } else {
        RiskBucket::Low
    }
}

fn pick_reaction_type<R: Rng>(rng: &mut R, types: &[String]) -> String {
    types[rng.gen_range(0..types.len())].clone()
}

fn pick_severity<R: Rng>(rng: &mut R, severe_probability: f64) -> Severity {
    let roll: f64 = rng.gen();
    if roll < severe_probability {
        Severity::Severe
    } else if roll < severe_probability + 0.35 {
        Severity::Moderate
    } else {
        Severity::Mild
    }
}
