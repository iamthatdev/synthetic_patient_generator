//! Content policy checking for dangerous terms and phrases.

use crate::domain::guardrail::{Violation, ViolationType};

/// Configuration for dangerous term categories
#[derive(Clone, Debug)]
pub struct DangerousTermsConfig {
    pub self_harm: Vec<String>,
    pub violence: Vec<String>,
    pub substance_abuse: Vec<String>,
}

impl Default for DangerousTermsConfig {
    fn default() -> Self {
        Self {
            self_harm: vec![
                "suicide".to_string(),
                "self-harm".to_string(),
                "suicidal ideation".to_string(),
                "overdose intentional".to_string(),
                "attempted suicide".to_string(),
            ],
            violence: vec![
                "homicide".to_string(),
                "murder".to_string(),
                "assault".to_string(),
                "abuse".to_string(),
                "domestic violence".to_string(),
            ],
            substance_abuse: vec![
                "alcohol abuse".to_string(),
                "drug abuse".to_string(),
                "addiction".to_string(),
                "withdrawal".to_string(),
            ],
        }
    }
}

/// Extract context around a found term
fn extract_context(text: &str, term: &str, context_chars: usize) -> String {
    if let Some(pos) = text.to_lowercase().find(&term.to_lowercase()) {
        let start = if pos > context_chars { pos - context_chars } else { 0 };
        let end = std::cmp::min(pos + term.len() + context_chars, text.len());
        text[start..end].to_string()
    } else {
        String::new()
    }
}

/// Check text for content policy violations
pub fn check_content_policy(
    text: &str,
    terms: &DangerousTermsConfig,
    patient_id: String,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let text_lower = text.to_lowercase();

    let categories: [(&str, &[String]); 3] = [
        ("self_harm", &terms.self_harm),
        ("violence", &terms.violence),
        ("substance_abuse", &terms.substance_abuse),
    ];

    for (category, patterns) in categories {
        for pattern in patterns {
            if text_lower.contains(&pattern.to_lowercase()) {
                let context = extract_context(text, pattern, 20);
                violations.push(Violation::new(
                    ViolationType::ContentPolicy {
                        category: category.to_string(),
                        term: pattern.clone(),
                        context,
                    },
                    patient_id.clone(),
                ));
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_self_harm_term() {
        let text = "Patient reports suicidal ideation";
        let config = DangerousTermsConfig::default();
        let violations = check_content_policy(text, &config, "P001".to_string());
        assert!(!violations.is_empty());
        assert_eq!(violations[0].severity(), crate::domain::guardrail::ViolationSeverity::Warning);
    }

    #[test]
    fn test_no_violations() {
        let text = "Patient has diabetes and hypertension";
        let config = DangerousTermsConfig::default();
        let violations = check_content_policy(text, &config, "P002".to_string());
        assert!(violations.is_empty());
    }

    #[test]
    fn test_extract_context() {
        let text = "Patient reports suicidal ideation and feelings of hopelessness";
        let context = extract_context(text, "suicidal ideation", 10);
        assert!(context.contains("suicidal ideation"));
    }

    #[test]
    fn test_multiple_violations() {
        let text = "Patient reports suicidal ideation and domestic violence";
        let config = DangerousTermsConfig::default();
        let violations = check_content_policy(text, &config, "P003".to_string());
        assert!(violations.len() >= 2);
    }
}
