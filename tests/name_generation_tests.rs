use synthetic_patient_data::domain::patient::PatientName;
use synthetic_patient_data::generation::generate_clinical_note_text;
use synthetic_patient_data::domain::patient::{Gender, PatientRecordDraft, PatientProfile, RiskBucket, Severity};

#[test]
fn test_patient_name_full() {
    let name = PatientName::new("John".to_string(), "Smith".to_string());
    assert_eq!(name.full(), "John Smith");
}

#[test]
fn test_patient_name_with_middle() {
    let name = PatientName::new("John".to_string(), "Smith".to_string())
        .with_middle('A');
    assert_eq!(name.full(), "John A. Smith");
}

#[test]
fn test_patient_name_with_suffix() {
    let name = PatientName::new("John".to_string(), "Smith".to_string())
        .with_suffix("Jr.".to_string());
    assert_eq!(name.full(), "John Smith, Jr.");
}

#[test]
fn test_patient_name_full_with_middle_and_suffix() {
    let name = PatientName::new("John".to_string(), "Smith".to_string())
        .with_middle('A')
        .with_suffix("Jr.".to_string());
    assert_eq!(name.full(), "John A. Smith, Jr.");
}

#[test]
fn test_patient_name_sort_key() {
    let name = PatientName::new("John".to_string(), "Smith".to_string());
    assert_eq!(name.sort_key(), "Smith, John");
}

#[test]
fn test_patient_name_initials() {
    let name = PatientName::new("John".to_string(), "Smith".to_string());
    assert_eq!(name.initials(), "J. Smith");

    let name_with_middle = PatientName::new("John".to_string(), "Smith".to_string())
        .with_middle('A');
    assert_eq!(name_with_middle.initials(), "J. A. Smith");
}

#[test]
fn test_clinical_note_includes_name() {
    let profile = PatientProfile {
        patient_id: "P00000001".to_string(),
        name: PatientName::new("Jane".to_string(), "Doe".to_string()),
        age: 45,
        gender: Gender::Female,
        region: "Northeast".to_string(),
        risk_bucket: RiskBucket::Low,
    };

    let draft = PatientRecordDraft {
        profile,
        comorbidities: vec!["diabetes".to_string()],
        medications: vec!["Metformin".to_string()],
        allergic_reaction: false,
        reaction_medication: None,
        reaction_type: None,
        reaction_severity: None,
    };

    let note = generate_clinical_note_text(&draft);

    assert!(note.contains("Jane Doe"), "Note should include patient name");
    assert!(!note.contains("P00000001"), "Note should not include patient ID when name is present");
}
