//! Uniqueness checking for patient records.

use crate::domain::guardrail::{Violation, ViolationType};
use crate::domain::patient::PatientRecord;
use std::collections::HashSet;

/// Check for duplicate patient IDs within a batch
pub fn check_uniqueness(records: &[PatientRecord]) -> Vec<Violation> {
    let mut seen_ids = HashSet::new();
    let mut duplicates = Vec::new();

    for record in records {
        if !seen_ids.insert(&record.patient_id) {
            duplicates.push(Violation::new(
                ViolationType::Uniqueness {
                    duplicate_id: record.patient_id.clone(),
                },
                record.patient_id.clone(),
            ));
        }
    }

    duplicates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::patient::{Gender, PatientMetadata, PatientName};

    fn make_test_record(id: &str) -> PatientRecord {
        PatientRecord {
            patient_id: id.to_string(),
            name: PatientName::new("Test".to_string(), "Patient".to_string()),
            age: 45,
            gender: Gender::Female,
            region: "Northeast".to_string(),
            comorbidities: vec![],
            medications: vec![],
            allergic_reaction: false,
            reaction_medication: None,
            reaction_type: None,
            reaction_severity: None,
            clinical_notes: vec![],
            metadata: PatientMetadata { seed: 0, batch_id: 0 },
        }
    }

    #[test]
    fn test_no_duplicates() {
        let records = vec![
            make_test_record("P001"),
            make_test_record("P002"),
            make_test_record("P003"),
        ];
        let violations = check_uniqueness(&records);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_with_duplicates() {
        let records = vec![
            make_test_record("P001"),
            make_test_record("P002"),
            make_test_record("P001"),
        ];
        let violations = check_uniqueness(&records);
        assert_eq!(violations.len(), 1);
        if let ViolationType::Uniqueness { duplicate_id } = &violations[0].violation_type {
            assert_eq!(duplicate_id, "P001");
        }
    }

    #[test]
    fn test_multiple_duplicates() {
        let records = vec![
            make_test_record("P001"),
            make_test_record("P002"),
            make_test_record("P001"),
            make_test_record("P002"),
            make_test_record("P003"),
        ];
        let violations = check_uniqueness(&records);
        assert_eq!(violations.len(), 2);
    }
}
