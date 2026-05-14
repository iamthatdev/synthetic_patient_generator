//! Statistical distribution checking for generated patient records.

use crate::domain::guardrail::{Violation, ViolationType};
use crate::domain::patient::PatientRecord;

/// Extension trait to make condition checking easier
pub trait PatientRecordExt {
    fn has_condition(&self, condition: &str) -> bool;
}

impl PatientRecordExt for PatientRecord {
    fn has_condition(&self, condition: &str) -> bool {
        self.comorbidities.iter().any(|c| c.eq_ignore_ascii_case(condition))
    }
}

/// Configuration for condition probabilities used in distribution checking
#[derive(Clone, Debug)]
pub struct ConditionsConfig {
    pub diabetes: f64,
    pub hypertension: f64,
    pub asthma: f64,
    pub chronic_kidney_disease: f64,
    pub coronary_artery_disease: f64,
    pub copd: f64,
    pub obesity: f64,
}

impl Default for ConditionsConfig {
    fn default() -> Self {
        Self {
            diabetes: 0.12,
            hypertension: 0.28,
            asthma: 0.09,
            chronic_kidney_disease: 0.04,
            coronary_artery_disease: 0.06,
            copd: 0.05,
            obesity: 0.22,
        }
    }
}

impl From<&crate::config::ConditionsConfig> for ConditionsConfig {
    fn from(c: &crate::config::ConditionsConfig) -> Self {
        Self {
            diabetes: c.diabetes,
            hypertension: c.hypertension,
            asthma: c.asthma,
            chronic_kidney_disease: c.chronic_kidney_disease,
            coronary_artery_disease: c.coronary_artery_disease,
            copd: c.copd,
            obesity: c.obesity,
        }
    }
}

/// Check if the distribution of conditions in a batch matches configured probabilities
pub fn check_distribution(
    records: &[PatientRecord],
    config: &ConditionsConfig,
    tolerance: f64,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let count = records.len() as f64;

    if count == 0.0 {
        return violations;
    }

    let checks: [(&str, f64, fn(&PatientRecord) -> bool); 7] = [
        ("diabetes", config.diabetes, |r| r.has_condition("diabetes")),
        ("hypertension", config.hypertension, |r| r.has_condition("hypertension")),
        ("asthma", config.asthma, |r| r.has_condition("asthma")),
        ("chronic_kidney_disease", config.chronic_kidney_disease, |r| r.has_condition("chronic_kidney_disease")),
        ("coronary_artery_disease", config.coronary_artery_disease, |r| r.has_condition("coronary_artery_disease")),
        ("copd", config.copd, |r| r.has_condition("copd")),
        ("obesity", config.obesity, |r| r.has_condition("obesity")),
    ];

    for (name, expected_prob, predicate) in checks {
        let actual_count = records.iter().filter(|r| predicate(r)).count();
        let actual_prob = actual_count as f64 / count;
        let deviation = (actual_prob - expected_prob).abs();

        if deviation > tolerance {
            violations.push(Violation::new(
                ViolationType::Distribution {
                    metric: format!("{}_rate", name),
                    expected: expected_prob,
                    actual: actual_prob,
                    deviation,
                },
                "aggregate".to_string(),
            ));
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::patient::{Gender, PatientMetadata, PatientName};

    fn make_test_record(id: &str, comorbidities: Vec<&str>) -> PatientRecord {
        PatientRecord {
            patient_id: id.to_string(),
            name: PatientName::new("Test".to_string(), "Patient".to_string()),
            age: 45,
            gender: Gender::Female,
            region: "Northeast".to_string(),
            comorbidities: comorbidities.into_iter().map(String::from).collect(),
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
    fn test_distribution_within_tolerance() {
        let config = ConditionsConfig::default();

        let mut records = Vec::new();
        for i in 0..100 {
            let comorbidities = if i < 12 { vec!["diabetes"] } else { vec![] };
            records.push(make_test_record(&format!("P{:03}", i), comorbidities));
        }

        let violations = check_distribution(&records, &config, 0.05);
        // diabetes is exactly 12%, within tolerance of 0.05 from 0.12
        assert!(violations.is_empty() || !violations.iter().any(|v| {
            matches!(&v.violation_type, ViolationType::Distribution { metric, .. } if metric == "diabetes_rate")
        }));
    }

    #[test]
    fn test_distribution_outside_tolerance() {
        let config = ConditionsConfig::default();

        let mut records = Vec::new();
        for i in 0..100 {
            let comorbidities = if i < 25 { vec!["diabetes"] } else { vec![] };
            records.push(make_test_record(&format!("P{:03}", i), comorbidities));
        }

        let violations = check_distribution(&records, &config, 0.05);
        assert!(!violations.is_empty());
    }
}
