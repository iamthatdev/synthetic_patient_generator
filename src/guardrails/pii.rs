//! PII (Personally Identifiable Information) detection and redaction.

use crate::domain::guardrail::{Violation, ViolationType};
use regex::Regex;
use std::sync::OnceLock;

struct PiiPattern {
    name: &'static str,
    regex: &'static str,
    redaction_template: &'static str,
}

fn pii_patterns() -> &'static [PiiPattern] {
    &[
        PiiPattern {
            name: "SSN",
            regex: r"\b\d{3}-\d{2}-\d{4}\b",
            redaction_template: "[REDACTED_SSN]",
        },
        PiiPattern {
            name: "SSN_NO_DASH",
            regex: r"\b\d{3}\s?\d{2}\s?\d{4}\b",
            redaction_template: "[REDACTED_SSN]",
        },
        PiiPattern {
            name: "PHONE",
            regex: r"\b\d{3}-\d{3}-\d{4}\b",
            redaction_template: "[REDACTED_PHONE]",
        },
        PiiPattern {
            name: "EMAIL",
            regex: r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
            redaction_template: "[REDACTED_EMAIL]",
        },
        PiiPattern {
            name: "ZIP_PLUS_4",
            regex: r"\b\d{5}-\d{4}\b",
            redaction_template: "[REDACTED_ZIP]",
        },
        PiiPattern {
            name: "CREDIT_CARD",
            regex: r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b",
            redaction_template: "[REDACTED_CARD]",
        },
    ]
}

fn cached_regex() -> &'static Vec<(&'static str, Regex)> {
    static CACHE: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        pii_patterns()
            .iter()
            .map(|p| (p.name, Regex::new(p.regex).expect("Invalid regex")))
            .collect()
    })
}

fn get_pattern_by_name(name: &str) -> &'static PiiPattern {
    pii_patterns().iter().find(|p| p.name == name).expect("Pattern not found")
}

/// Result of PII scanning
#[derive(Clone, Debug)]
pub struct PiiScanResult {
    pub redacted_text: String,
    pub violations: Vec<Violation>,
}

/// Scan text for PII patterns and return redacted text with violations
pub fn scan_and_redact_pii(text: &str, patient_id: String) -> PiiScanResult {
    let mut result = text.to_string();
    let mut violations = Vec::new();

    for (pattern_name, regex) in cached_regex().iter() {
        let pattern = get_pattern_by_name(pattern_name);
        let matches: Vec<_> = regex.find_iter(&result).collect();
        if matches.is_empty() {
            continue;
        }

        for m in matches {
            violations.push(Violation::new(
                ViolationType::PiiScan {
                    pattern_type: pattern.name.to_string(),
                    position: m.start(),
                    redacted_to: pattern.redaction_template.to_string(),
                },
                patient_id.clone(),
            ));
        }

        result = regex.replace_all(&result, pattern.redaction_template).to_string();
    }

    PiiScanResult {
        redacted_text: result,
        violations,
    }
}

/// Check if text contains any PII patterns (without redacting)
pub fn contains_pii(text: &str) -> bool {
    for (_, regex) in cached_regex().iter() {
        if regex.is_match(text) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ssn() {
        let text = "Patient SSN is 123-45-6789";
        assert!(contains_pii(text));
    }

    #[test]
    fn test_redact_ssn() {
        let text = "Patient SSN is 123-45-6789";
        let result = scan_and_redact_pii(text, "P001".to_string());
        assert_eq!(result.redacted_text, "Patient SSN is [REDACTED_SSN]");
        assert_eq!(result.violations.len(), 1);
    }

    #[test]
    fn test_detect_email() {
        let text = "Contact user@example.com";
        assert!(contains_pii(text));
    }

    #[test]
    fn test_redact_email() {
        let text = "Email: user@example.com";
        let result = scan_and_redact_pii(text, "P002".to_string());
        assert_eq!(result.redacted_text, "Email: [REDACTED_EMAIL]");
    }

    #[test]
    fn test_no_pii() {
        let text = "Patient has diabetes and hypertension";
        assert!(!contains_pii(text));
        let result = scan_and_redact_pii(text, "P003".to_string());
        assert_eq!(result.redacted_text, text);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_multiple_pii_types() {
        let text = "SSN: 123-45-6789, Phone: 555-123-4567";
        let result = scan_and_redact_pii(text, "P004".to_string());
        assert_eq!(result.violations.len(), 2);
        assert!(result.redacted_text.contains("[REDACTED_SSN]"));
        assert!(result.redacted_text.contains("[REDACTED_PHONE]"));
    }
}
