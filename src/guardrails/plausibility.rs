//! Medical plausibility checking for patient records.

use crate::domain::guardrail::{Violation, ViolationType};
use crate::domain::patient::{Gender, PatientRecord};

/// Check if a patient record has medically implausible combinations
pub fn check_medical_plausibility(record: &PatientRecord) -> Vec<Violation> {
    let mut violations = Vec::new();

    let max_comorbidities = if record.age < 30 {
        4
    } else if record.age < 65 {
        6
    } else {
        10
    };

    if record.comorbidities.len() > max_comorbidities {
        violations.push(Violation::new(
            ViolationType::Plausibility {
                rule: "too_many_comorbidities".to_string(),
                details: format!(
                    "Age {} has {} comorbidities (max: {})",
                    record.age,
                    record.comorbidities.len(),
                    max_comorbidities
                ),
            },
            record.patient_id.clone(),
        ));
    }

    if record.gender == Gender::Male {
        let male_incompatible = ["ovarian_cancer", "cervical_cancer", "uterine_cancer"];
        for condition in &male_incompatible {
            if record.comorbidities.iter().any(|c| c.eq_ignore_ascii_case(condition)) {
                violations.push(Violation::new(
                    ViolationType::Plausibility {
                        rule: "gender_condition_mismatch".to_string(),
                        details: format!("Male patient has {}", condition),
                    },
                    record.patient_id.clone(),
                ));
            }
        }
    }

    if record.gender == Gender::Female {
        let female_incompatible = ["prostate_cancer", "testicular_cancer"];
        for condition in &female_incompatible {
            if record.comorbidities.iter().any(|c| c.eq_ignore_ascii_case(condition)) {
                violations.push(Violation::new(
                    ViolationType::Plausibility {
                        rule: "gender_condition_mismatch".to_string(),
                        details: format!("Female patient has {}", condition),
                    },
                    record.patient_id.clone(),
                ));
            }
        }
    }

    if record.medications.iter().any(|m| m.eq_ignore_ascii_case("metformin"))
        && !record.comorbidities.iter().any(|c| c.eq_ignore_ascii_case("diabetes"))
    {
        violations.push(Violation::new(
            ViolationType::Plausibility {
                rule: "medication_without_condition".to_string(),
                details: "Metformin prescribed without diabetes diagnosis".to_string(),
            },
            record.patient_id.clone(),
        ));
    }

    if record.age < 18 && record.medications.iter().any(|m| m.contains("Lisinopril")) {
        violations.push(Violation::new(
            ViolationType::Plausibility {
                rule: "age_inappropriate_medication".to_string(),
                details: format!("Age {} prescribed Lisinopril (adult medication)", record.age),
            },
            record.patient_id.clone(),
        ));
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::patient::{PatientMetadata, PatientName};

    fn make_test_record(age: u8, gender: Gender, comorbidities: Vec<&str>, medications: Vec<&str>) -> PatientRecord {
        PatientRecord {
            patient_id: "P001".to_string(),
            name: PatientName::new("Test".to_string(), "Patient".to_string()),
            age,
            gender,
            region: "Northeast".to_string(),
            comorbidities: comorbidities.into_iter().map(String::from).collect(),
            medications: medications.into_iter().map(String::from).collect(),
            allergic_reaction: false,
            reaction_medication: None,
            reaction_type: None,
            reaction_severity: None,
            clinical_notes: vec![],
            metadata: PatientMetadata { seed: 0, batch_id: 0 },
        }
    }

    #[test]
    fn test_young_patient_too_many_comorbidities() {
        let record = make_test_record(
            25,
            Gender::Female,
            vec!["diabetes", "hypertension", "asthma", "copd", "obesity", "ckd"],
            vec![],
        );
        let violations = check_medical_plausibility(&record);
        assert!(!violations.is_empty());
        if let ViolationType::Plausibility { rule, .. } = &violations[0].violation_type {
            assert_eq!(rule, "too_many_comorbidities");
        }
    }

    #[test]
    fn test_male_with_ovarian_cancer() {
        let record = make_test_record(
            50,
            Gender::Male,
            vec!["ovarian_cancer"],
            vec![],
        );
        let violations = check_medical_plausibility(&record);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_metformin_without_diabetes() {
        let record = make_test_record(
            45,
            Gender::Female,
            vec!["hypertension"],
            vec!["Metformin"],
        );
        let violations = check_medical_plausibility(&record);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_valid_record() {
        let record = make_test_record(
            45,
            Gender::Female,
            vec!["diabetes", "hypertension"],
            vec!["Metformin", "Lisinopril"],
        );
        let violations = check_medical_plausibility(&record);
        assert!(violations.is_empty());
    }
}
