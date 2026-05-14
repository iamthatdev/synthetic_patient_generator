//! Domain types for the guardrail system.

use serde::{Deserialize, Serialize};

/// Severity level for guardrail violations
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViolationSeverity {
    Warning,
    Error,
}

impl std::fmt::Display for ViolationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationSeverity::Warning => write!(f, "Warning"),
            ViolationSeverity::Error => write!(f, "Error"),
        }
    }
}

/// Types of violations that can be detected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationType {
    /// PII pattern detected (SSN, phone, email, etc.)
    PiiScan {
        pattern_type: String,
        position: usize,
        redacted_to: String,
    },
    /// Name matches blocked list
    NameSafety {
        blocked_name: String,
    },
    /// Content policy violation (dangerous terms)
    ContentPolicy {
        category: String,
        term: String,
        context: String,
    },
    /// Medical implausibility detected
    Plausibility {
        rule: String,
        details: String,
    },
    /// Statistical distribution deviation
    Distribution {
        metric: String,
        expected: f64,
        actual: f64,
        deviation: f64,
    },
    /// Duplicate ID detected
    Uniqueness {
        duplicate_id: String,
    },
}

impl ViolationType {
    pub fn severity(&self) -> ViolationSeverity {
        match self {
            ViolationType::PiiScan { .. } => ViolationSeverity::Error,
            ViolationType::NameSafety { .. } => ViolationSeverity::Error,
            ViolationType::ContentPolicy { .. } => ViolationSeverity::Warning,
            ViolationType::Plausibility { .. } => ViolationSeverity::Warning,
            ViolationType::Distribution { .. } => ViolationSeverity::Warning,
            ViolationType::Uniqueness { .. } => ViolationSeverity::Error,
        }
    }
}

/// A single guardrail violation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub patient_id: String,
    pub timestamp: String,
}

impl Violation {
    pub fn new(violation_type: ViolationType, patient_id: String) -> Self {
        Self {
            violation_type,
            patient_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn severity(&self) -> ViolationSeverity {
        self.violation_type.severity()
    }
}

/// Collection of violations from a batch
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ViolationBatch {
    pub count: usize,
    pub warnings: Vec<Violation>,
    pub errors: Vec<Violation>,
    pub rejected_ids: Vec<String>,
}

impl ViolationBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, patient_id: String, violations: Vec<Violation>) {
        for v in violations {
            match v.severity() {
                ViolationSeverity::Warning => self.warnings.push(v),
                ViolationSeverity::Error => self.errors.push(v),
            }
        }
        self.count += 1;
    }

    pub fn reject(&mut self, patient_id: String) {
        self.rejected_ids.push(patient_id);
    }

    pub fn merge(&mut self, other: &ViolationBatch) {
        self.warnings.extend(other.warnings.clone());
        self.errors.extend(other.errors.clone());
        self.rejected_ids.extend(other.rejected_ids.clone());
        self.count += other.count;
    }
}

/// Summary statistics for guardrail report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryStats {
    pub total_checked: usize,
    pub passed: usize,
    pub flagged: usize,
    pub rejected: usize,
}

impl SummaryStats {
    pub fn new() -> Self {
        Self {
            total_checked: 0,
            passed: 0,
            flagged: 0,
            rejected: 0,
        }
    }

    pub fn add_batch(&mut self, batch: &ViolationBatch) {
        self.total_checked += batch.count;
        self.flagged += batch.warnings.len() + batch.errors.len();
        self.rejected += batch.rejected_ids.len();
        self.passed = self.total_checked - self.rejected;
    }
}

/// Check-specific results for the guardrail report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PiiScanResult {
    pub enabled: bool,
    pub triggered: usize,
    pub violations: Vec<String>,
}

impl Default for PiiScanResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            violations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NameSafetyResult {
    pub enabled: bool,
    pub triggered: usize,
    pub blocked_names_found: Vec<String>,
}

impl Default for NameSafetyResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            blocked_names_found: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentPolicyResult {
    pub enabled: bool,
    pub triggered: usize,
    pub violations_by_category: std::collections::HashMap<String, usize>,
    pub examples: Vec<PolicyViolationExample>,
}

impl Default for ContentPolicyResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            violations_by_category: std::collections::HashMap::new(),
            examples: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyViolationExample {
    pub patient_id: String,
    pub category: String,
    pub term: String,
    pub context: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlausibilityResult {
    pub enabled: bool,
    pub triggered: usize,
    pub violations_by_type: std::collections::HashMap<String, usize>,
}

impl Default for PlausibilityResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            violations_by_type: std::collections::HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributionResult {
    pub enabled: bool,
    pub triggered: usize,
    pub deviations: Vec<DistributionDeviation>,
}

impl Default for DistributionResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            deviations: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributionDeviation {
    pub metric: String,
    pub expected: f64,
    pub actual: f64,
    pub deviation: f64,
    pub within_tolerance: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UniquenessResult {
    pub enabled: bool,
    pub triggered: usize,
    pub duplicate_ids: Vec<String>,
}

impl Default for UniquenessResult {
    fn default() -> Self {
        Self {
            enabled: true,
            triggered: 0,
            duplicate_ids: Vec::new(),
        }
    }
}

/// All check results combined
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResults {
    pub pii_scan: PiiScanResult,
    pub name_safety: NameSafetyResult,
    pub content_policy: ContentPolicyResult,
    pub plausibility: PlausibilityResult,
    pub distribution: DistributionResult,
    pub uniqueness: UniquenessResult,
}

impl Default for CheckResults {
    fn default() -> Self {
        Self {
            pii_scan: PiiScanResult::default(),
            name_safety: NameSafetyResult::default(),
            content_policy: ContentPolicyResult::default(),
            plausibility: PlausibilityResult::default(),
            distribution: DistributionResult::default(),
            uniqueness: UniquenessResult::default(),
        }
    }
}

/// Complete guardrail report
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardrailReport {
    pub job_id: String,
    pub timestamp: String,
    pub summary: SummaryStats,
    pub checks: CheckResults,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_check_time_ms: u64,
    pub avg_time_per_record_ms: f64,
}

impl GuardrailReport {
    pub fn new(job_id: String) -> Self {
        Self {
            job_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
            summary: SummaryStats::new(),
            checks: CheckResults::default(),
            performance_metrics: PerformanceMetrics {
                total_check_time_ms: 0,
                avg_time_per_record_ms: 0.0,
            },
        }
    }
}
