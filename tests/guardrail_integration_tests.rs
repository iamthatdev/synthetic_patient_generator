//! Integration tests for the guardrail system.

use synthetic_patient_data::domain::patient::{Gender, PatientMetadata, PatientName, PatientRecord};
use synthetic_patient_data::guardrails::distribution;
use synthetic_patient_data::guardrails::plausibility;
use synthetic_patient_data::guardrails::uniqueness;
use synthetic_patient_data::guardrails::pii;

fn create_test_record(
    id: &str,
    age: u8,
    comorbidities: Vec<&str>,
    medications: Vec<&str>,
) -> PatientRecord {
    PatientRecord {
        patient_id: id.to_string(),
        name: PatientName::new("Test".to_string(), "Patient".to_string()),
        age,
        gender: Gender::Female,
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

fn create_test_record_with_note(
    id: &str,
    age: u8,
    comorbidities: Vec<&str>,
    medications: Vec<&str>,
    note_text: &str,
) -> PatientRecord {
    use synthetic_patient_data::domain::clinical_note::ClinicalNote;

    let note = ClinicalNote {
        note_id: format!("note_{}_0", id),
        patient_id: id.to_string(),
        note_type: "primary_care".to_string(),
        text: note_text.to_string(),
    };

    let mut record = create_test_record(id, age, comorbidities, medications);
    record.clinical_notes = vec![note];
    record
}

#[test]
fn test_full_guardrail_pipeline() {
    let records = vec![
        create_test_record("P001", 45, vec!["diabetes"], vec!["Metformin"]),
        create_test_record_with_note("P002", 30, vec![], vec![], "Call 555-123-4567"),
        create_test_record("P003", 25, vec!["diabetes", "hypertension", "asthma", "copd", "obesity"], vec![]),
    ];

    let config = distribution::ConditionsConfig::default();

    // Run distribution check
    let violations = distribution::check_distribution(&records, &config, 0.05);
    println!("Distribution violations: {}", violations.len());

    // Run plausibility check
    for record in &records {
        let plaus_violations = plausibility::check_medical_plausibility(record);
        println!("Plausibility violations for {}: {}", record.patient_id, plaus_violations.len());
    }

    // Run uniqueness check
    let uniq_violations = uniqueness::check_uniqueness(&records);
    assert_eq!(uniq_violations.len(), 0, "Should have no duplicates");

    // Run PII check on notes
    let mut pii_count = 0;
    for record in &records {
        for note in &record.clinical_notes {
            let pii_result = pii::scan_and_redact_pii(&note.text, record.patient_id.clone());
            pii_count += pii_result.violations.len();
        }
    }
    // P002's note has a phone number
    assert!(pii_count > 0, "Should detect PII in P002's note");
}
