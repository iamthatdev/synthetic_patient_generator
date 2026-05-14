# Patient Names and Comprehensive Guardrails System

## Design Document

**Version:** 1.0
**Date:** 2026-05-13
**Status:** Design Phase
**Authors:** Synthetic Patient Data Generator Team

---

## Executive Summary

This document describes the implementation of two major feature additions to the Synthetic Patient Data Generator:

1. **Patient Name Generation** — Deterministic, reproducible synthetic names with regional and gender awareness
2. **Comprehensive Guardrails System** — Multi-layer validation, PII scanning, and policy enforcement integrated as a first-class pipeline stage

Together, these features enable demonstration of responsible AI practices in healthcare RAG applications, including:
- Multi-hop and recursive RAG with proper data provenance
- Privacy-preserving synthetic data generation
- Real-time PII detection and redaction
- Content safety and medical plausibility validation
- Statistical distribution verification

---

## Table of Contents

1. [Overview](#overview)
2. [Patient Name Generation System](#patient-name-generation-system)
3. [Guardrails System](#guardrails-system)
4. [Architecture Integration](#architecture-integration)
5. [Configuration](#configuration)
6. [Data Structures](#data-structures)
7. [Implementation Plan](#implementation-plan)
8. [Testing Strategy](#testing-strategy)
9. [Performance Considerations](#performance-considerations)

---

## Overview

### Goals

1. **Generate realistic synthetic patient names** that are deterministic (same seed → same names) while maintaining plausibility
2. **Implement comprehensive guardrails** to validate generated data at multiple levels
3. **Demonstrate responsible AI** patterns applicable to real healthcare RAG systems
4. **Maintain reproducibility** — all generation remains seed-deterministic
5. **Zero breaking changes** to the existing API (output formats extended, not replaced)

### Non-Goals

- Using LLMs for name generation (too slow, non-deterministic)
- Storing real patient data (synthetic only)
- Modifying existing patient_id semantics (still primary identifier)

---

## Patient Name Generation System

### Design Philosophy: Hybrid Deterministic Approach

We combine the best of multiple approaches:

| Approach | Contribution |
|----------|--------------|
| **Deterministic Lookup Tables** | Seed reproducibility, fast O(1) lookup |
| **Census Frequency Weighting** | Realistic name distribution (Smith > Zyzzyx) |
| **Regional Awareness** | Names vary by region (e.g., Garcia in Southwest) |
| **Gender Awareness** | Male/Female name pools |

### Name Data Structure

```rust
/// Represents a patient's synthetic name
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PatientName {
    pub first_name: String,
    pub last_name: String,
    pub middle_initial: Option<char>,
    pub name_suffix: Option<String>,  // Jr., Sr., II, III
}

impl PatientName {
    /// Returns full name formatted as "First M. Last, Suffix"
    pub fn full(&self) -> String { ... }

    /// Returns sortable name as "Last, First M."
    pub fn sort_key(&self) -> String { ... }
}
```

### Deterministic Name Generation

**Core Algorithm:**

```rust
// Input: rng (seeded), gender, region
// Output: (first_name, last_name)

fn generate_name(
    rng: &mut ChaCha8Rng,
    gender: &Gender,
    region: &str,
) -> PatientName {
    // 1. Select first name from gender-appropriate pool
    let first_pool = match gender {
        Gender::Female => &FIRST_NAMES_FEMALE,
        Gender::Male => &FIRST_NAMES_MALE,
    };
    let first_idx = rng.gen_range(0..first_pool.len());
    let first_name = first_pool[first_idx].clone();

    // 2. Select last name from region-weighted pool
    let last_pool = LAST_NAMES_BY_REGION.get_region_pool(region);
    let last_idx = rng.gen_range(0..last_pool.len());
    let last_name = last_pool[last_idx].clone();

    // 3. Optionally add middle initial (10% chance)
    let middle_initial = if rng.gen::<f64>() < 0.10 {
        Some((b'A' + rng.gen_range(0..26)) as char)
    } else {
        None
    };

    // 4. Optionally add suffix (rare, age-dependent)
    let name_suffix = generate_suffix(rng);

    PatientName { first_name, last_name, middle_initial, name_suffix }
}
```

### Name Data Sources

**File:** `src/data/names.rs` (~500 lines)

```rust
// ~100 most common female names by census frequency
pub static FIRST_NAMES_FEMALE: &[&str] = &[
    "Mary", "Patricia", "Jennifer", "Linda", "Elizabeth",
    "Barbara", "Susan", "Jessica", "Sarah", "Karen",
    // ... 90 more
];

// ~100 most common male names by census frequency
pub static FIRST_NAMES_MALE: &[&str] = &[
    "James", "Robert", "John", "Michael", "David",
    "William", "Richard", "Joseph", "Thomas", "Christopher",
    // ... 90 more
];

// ~200 last names by census frequency
pub static LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones",
    "Garcia", "Miller", "Davis", "Rodriguez", "Martinez",
    // ... 190 more
];

// Regional surname adjustments (demonstrates demographic awareness)
pub static LAST_NAMES_BY_REGION: RegionalNamePools = RegionalNamePools {
    northeast: &["Smith", "Johnson", "Williams", ...],
    southeast: &["Smith", "Johnson", "Williams", ...],
    midwest: &["Smith", "Johnson", "Williams", ...],
    southwest: &["Garcia", "Rodriguez", "Martinez", "Smith", ...],  // Higher Hispanic
    west: &["Smith", "Johnson", "Williams", "Garcia", ...],
};
```

### Reproducibility Impact

**Important:** Adding names introduces 2-3 new RNG draws per patient:

| Change | Impact |
|--------|--------|
| Before | Patient generation used ~5 RNG draws |
| After | Patient generation uses ~7-8 RNG draws |
| Result | Same seed produces different patient data (names shift all subsequent draws) |

**Mitigation:** Document this as a data format change. The API remains compatible, but output hashes will differ.

### Integration Points

| File | Change |
|------|--------|
| `src/domain/patient.rs` | Add `name: PatientName` to `PatientProfile` |
| `src/generation.rs` | Add `generate_name()` function |
| `src/actors/note.rs` | Update clinical note template to use names |
| `src/eval_generation.rs` | Optionally include names in evidence queries |
| `src/data/names.rs` | **NEW** — name data tables |

---

## Guardrails System

### Design Philosophy: Guardrails as a Pipeline Stage

The key insight: **guardrails are a cross-cutting concern best implemented as a dedicated actor stage**.

```
Before:
ProfileActor → ConditionActor → MedicationActor → ReactionActor → NoteActor
                                                                     ↓
                                                              ChunkingActor → WriterActor

After:
ProfileActor → ConditionActor → MedicationActor → ReactionActor → NoteActor
                                                                     ↓
                                                              GuardrailActor ← NEW
                                                                     ↓
                                                              ChunkingActor → WriterActor
```

**Why this approach:**

1. **Separation of concerns** — Guardrail logic is isolated
2. **Composability** — Can be added/removed without touching other actors
3. **Observability** — All violations flow through one point
4. **Testability** — Guardrail checks are pure functions
5. **Showcase value** — Demonstrates architectural pattern

### Guardrail Categories

#### 1. PII Detection and Redaction

**Purpose:** Ensure no real PII leaks into generated data

**Patterns Detected:**

| Pattern | Regex | Example |
|---------|-------|---------|
| SSN | `\d{3}-\d{2}-\d{4}` | `123-45-6789` |
| Phone | `\d{3}-\d{3}-\d{4}` | `555-123-4567` |
| Email | `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}` | `user@example.com` |
| ZIP+4 | `\d{5}-\d{4}` | `12345-6789` |
| Credit Card | `\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}` | `1234 5678 9012 3456` |

**Action:** Redact with placeholder, log violation, flag record

```rust
pub fn scan_and_redact_pii(text: &str) -> (String, Vec<PiiViolation>) {
    let mut result = text.to_string();
    let mut violations = Vec::new();

    for pattern in PII_PATTERNS {
        while let Some(match_) = pattern.find(&result) {
            violations.push(PiiViolation {
                pattern_type: pattern.name,
                position: match_.start(),
                redacted_to: format!("[REDACTED_{}]", pattern.name),
            });
            result.replace_range(match_.range(), &violations.last().unwrap().redacted_to);
        }
    }

    (result, violations)
}
```

#### 2. Name Safety Check

**Purpose:** Prevent generated names from matching real public figures

**Implementation:**

```rust
pub static BLOCKED_NAMES: HashSet<&str> = HashSet::from([
    // Politicians
    "Biden", "Trump", "Harris", "Obama",
    // Healthcare figures (to avoid confusion)
    "Fauci", "CDC", "FDA",
    // Celebrities (optional, can be configured)
    // ...
]);

pub fn check_name_safety(name: &PatientName) -> Vec<NameViolation> {
    let mut violations = Vec::new();

    if BLOCKED_NAMES.contains(name.last_name.as_str()) {
        violations.push(NameViolation::BlockedSurname(name.last_name.clone()));
    }

    // Check first+last combinations
    let full = format!("{} {}", name.first_name, name.last_name);
    if BLOCKED_FULL_NAMES.contains(full.as_str()) {
        violations.push(NameViolation::BlockedFullName(full));
    }

    violations
}
```

#### 3. Content Policy Check

**Purpose:** Flag potentially dangerous content in clinical notes

**Configurable Terms:**

```toml
[guardrails.dangerous_terms]
# Categories of terms to flag
self_harm = ["suicide", "self-harm", "suicidal ideation", "overdose intentional"]
violence = ["homicide", "murder", "assault", "abuse"]
discrimination = ["racial slur", "discrimination"]  # Demonstration only
```

**Action:** Flag with warning, but allow (clinical reality includes these terms)

```rust
pub fn check_content_policy(note_text: &str, terms: &DangerousTermsConfig) -> Vec<PolicyViolation> {
    let lower = note_text.to_lowercase();
    let mut violations = Vec::new();

    for (category, patterns) in terms.iter() {
        for pattern in patterns {
            if lower.contains(pattern) {
                violations.push(PolicyViolation {
                    category: category.clone(),
                    term_found: pattern.clone(),
                    context: extract_context(note_text, pattern, 20),  // 20 chars before/after
                });
            }
        }
    }

    violations
}
```

#### 4. Medical Plausibility Check

**Purpose:** Flag medically implausible combinations

**Rules:**

| Rule | Description | Severity |
|------|-------------|----------|
| Age-Condition Mismatch | 18-year-old with 7 comorbidities | Warning |
| Gender-Condition Mismatch | Male with ovarian cancer | Error |
| Medication-Condition Mismatch | No diabetes but on Metformin | Warning |
| Age-Inappropriate Medication | Pediatric drug for 80-year-old | Warning |

```rust
pub fn check_medical_plausibility(record: &PatientRecord) -> Vec<PlausibilityViolation> {
    let mut violations = Vec::new();

    // Rule: Too many comorbidities for age
    if record.comorbidities.len() > 5 && record.age < 30 {
        violations.push(PlausibilityViolation::UnlikelyComorbidityCount {
            age: record.age,
            count: record.comorbidities.len(),
        });
    }

    // Rule: Gender-specific conditions
    if record.gender == Gender::Male && record.comorbidities.contains(&"ovarian_cancer".to_string()) {
        violations.push(PlausibilityViolation::GenderConditionMismatch {
            gender: "Male".to_string(),
            condition: "ovarian_cancer".to_string(),
        });
    }

    // Rule: Medication requires condition
    if record.medications.contains(&"Metformin".to_string())
        && !record.comorbidities.contains(&"diabetes".to_string()) {
        violations.push(PlausibilityViolation::MedicationWithoutCondition {
            medication: "Metformin".to_string(),
            expected_condition: "diabetes".to_string(),
        });
    }

    violations
}
```

#### 5. Statistical Distribution Check

**Purpose:** Verify generated data matches configured probabilities

**Implementation:** Batch-level check after each N records

```rust
pub fn check_distribution(
    batch: &[PatientRecord],
    config: &ConditionsConfig,
    tolerance: f64,
) -> Vec<DistributionViolation> {
    let count = batch.len();
    let mut violations = Vec::new();

    // Check diabetes rate
    let diabetes_count = batch.iter().filter(|r| r.has_condition("diabetes")).count();
    let actual_rate = diabetes_count as f64 / count as f64;
    let expected_rate = config.diabetes;

    if (actual_rate - expected_rate).abs() > tolerance {
        violations.push(DistributionViolation::RateDeviation {
            condition: "diabetes".to_string(),
            expected: expected_rate,
            actual: actual_rate,
            deviation: (actual_rate - expected_rate).abs(),
        });
    }

    // Repeat for other conditions...

    violations
}
```

#### 6. Uniqueness Check

**Purpose:** Ensure no duplicate IDs within and across batches

**Implementation:**

```rust
pub fn check_uniqueness(batch: &[PatientRecord]) -> Vec<UniquenessViolation> {
    let mut seen_ids = HashSet::new();
    let mut duplicates = Vec::new();

    for record in batch {
        if !seen_ids.insert(&record.patient_id) {
            duplicates.push(UniquenessViolation::DuplicatePatientId(record.patient_id.clone()));
        }
    }

    duplicates
}
```

### Guardrail Actor

**File:** `src/actors/guardrail.rs`

```rust
pub struct GuardrailActor {
    config: GuardrailConfig,
    violation_counts: Arc<Mutex<ViolationCounts>>,
}

#[actor]
impl GuardrailActor {
    async fn handle_batch(&mut self, batch: Vec<PatientRecord>) -> GuardrailResult {
        let mut results = Vec::new();
        let mut batch_violations = ViolationBatch::new();

        for record in batch {
            let mut violations = Vec::new();

            // Run all checks
            violations.extend(check_name_safety(&record.profile.name));
            violations.extend(scan_and_redact_pii(&record.clinical_notes_text()));
            violations.extend(check_content_policy(&record.clinical_notes_text(), &self.config.content_policy));
            violations.extend(check_medical_plausibility(&record));

            // Determine disposition
            let disposition = if violations.iter().any(|v| v.severity() == Severity::Error) {
                Disposition::Rejected
            } else if !violations.is_empty() {
                Disposition::Flagged(violations)
            } else {
                Disposition::Passed
            };

            match disposition {
                Disposition::Passed => results.push(record),
                Disposition::Flagged(v) => {
                    batch_violations.add(record.patient_id.clone(), v);
                    results.push(record);  // Still pass, but flagged
                }
                Disposition::Rejected => {
                    batch_violations.rejected(record.patient_id.clone());
                }
            }
        }

        // Update global counters
        self.violation_counts.lock().await.merge(&batch_violations);

        // Forward passed records to ChunkingActor
        Ok(results)
    }
}
```

### Guardrail Report Output

**File:** `data/guardrail_report.json`

```json
{
  "job_id": "job_1778689198760",
  "timestamp": "2026-05-13T12:34:56Z",
  "summary": {
    "total_checked": 100000,
    "passed": 99847,
    "flagged": 153,
    "rejected": 0
  },
  "checks": {
    "pii_scan": {
      "enabled": true,
      "triggered": 0,
      "violations": []
    },
    "name_safety": {
      "enabled": true,
      "triggered": 0,
      "blocked_names_found": []
    },
    "content_policy": {
      "enabled": true,
      "triggered": 3,
      "violations_by_category": {
        "self_harm": 2,
        "violence": 1
      },
      "examples": [
        {
          "patient_id": "P00001234",
          "category": "self_harm",
          "term": "suicide",
          "context": "...patient reported suicidal ideation..."
        }
      ]
    },
    "plausibility": {
      "enabled": true,
      "triggered": 47,
      "violations_by_type": {
        "too_many_comorbidities": 31,
        "impossible_age_condition": 16
      }
    },
    "distribution": {
      "enabled": true,
      "triggered": 2,
      "deviations": [
        {
          "metric": "diabetes_rate",
          "expected": 0.12,
          "actual": 0.127,
          "deviation": 0.007,
          "within_tolerance": true
        }
      ]
    },
    "uniqueness": {
      "enabled": true,
      "triggered": 0,
      "duplicate_ids": []
    }
  },
  "performance_metrics": {
    "total_check_time_ms": 234,
    "avg_time_per_record_ms": 0.0023
  }
}
```

---

## Architecture Integration

### Updated Actor Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLI / Config                                   │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         OrchestratorActor                                   │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                      Patient Generation Pipeline                      │  │
│  │                                                                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │  │
│  │  │  Profile │→ │Condition │→ │Medication│→ │ Reaction │→ │  Note  │  │  │
│  │  │  Actor   │  │  Actor   │  │  Actor   │  │  Actor   │  │ Actor  │  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └────┬───┘  │  │
│  │                                                           │         │  │
│  │                                                           ▼         │  │
│  │                                                    ┌───────────┐    │  │
│  │                                                    │Guardrail  │    │  │
│  │                                                    │   Actor   │    │  │
│  │                                                    └─────┬─────┘    │  │
│  │                                                          │         │  │
│  │                                                          ▼         │  │
│  │                                                   ┌──────────┐    │  │
│  │                                                   │Chunking  │    │  │
│  │                                                   │  Actor   │    │  │
│  │                                                   └────┬─────┘    │  │
│  │                                                        │          │  │
│  │                                                        ▼          │  │
│  │                                                   ┌──────────┐    │  │
│  │                                                   │  Writer  │    │  │
│  │                                                   │  Actor   │    │  │
│  │                                                   └──────────┘    │  │
│  │                                                          │         │  │
│  │                                                          ▼         │  │
│  │                                                   ┌──────────┐    │  │
│  │                                                   │ Reports  │    │  │
│  │                                                   │Generated │    │  │
│  │                                                   └──────────┘    │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                       Eval Generation Pipeline                        │  │
│  │                    (after patient pipeline)                          │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Message Flow

```rust
// src/actors/messages.rs - Additions

pub enum PipelineMsg {
    // Existing messages...
    ProfileGenerated(ProfileResult),
    ConditionsAssigned(PatientWithConditions),
    MedicationsAssigned(PatientWithMedications),
    ReactionSimulated(PatientRecordDraft),
    ClinicalNoteGenerated(PatientRecordDraft),

    // NEW: Guardrail messages
    GuardrailCheckRequest {
        records: Vec<PatientRecord>,
        reply_to: ActorCell,
    },
    GuardrailCheckResult {
        records: Vec<PatientRecord>,
        violations: ViolationBatch,
    },

    // Existing messages continue...
    ChunkGenerated(ChunkResult),
    BatchWritten(BatchWrittenMsg),
}
```

---

## Configuration

### New Config Sections

**File:** `config/default.toml` (additions)

```toml
# ============================================
# Patient Name Generation
# ============================================
[names]
# Enable/disable name generation
enabled = true

# Use regionally-aware name pools
region_aware = true

# Probability of including middle initial
middle_initial_probability = 0.10

# Probability of including suffix (Jr., Sr., II, III)
suffix_probability = 0.02

# Suffix probability increases with age
age_scaled_suffix = true

# ============================================
# Guardrails Configuration
# ============================================
[guardrails]
# Master enable/disable for all guardrails
enabled = true

# If true, reject records with errors; if false, flag only
fail_on_error = false

# Generate guardrail report
generate_report = true

# ============================================
# PII Detection
# ============================================
[guardrails.pii]
enabled = true
# Action: "redact", "flag", "reject"
action = "redact"
# Patterns to detect (all enabled by default)
detect_ssn = true
detect_phone = true
detect_email = true
detect_zip_plus_4 = true
detect_credit_card = true

# ============================================
# Name Safety
# ============================================
[guardrails.name_safety]
enabled = true
# Action: "reject", "regenerate", "flag"
action = "reject"
# Additional blocked name regex patterns
blocked_patterns = []
# Block full names of public figures
block_public_figures = true

# ============================================
# Content Policy
# ============================================
[guardrails.content_policy]
enabled = true

# Terms to flag (not reject, just flag for review)
# These are clinical terms that may appear in real notes
[guardrails.content_policy.dangerous_terms]
self_harm = ["suicide", "self-harm", "suicidal ideation", "overdose intentional", "attempted suicide"]
violence = ["homicide", "murder", "assault", "abuse", "domestic violence"]
substance_abuse = ["alcohol abuse", "drug abuse", "addiction", "withdrawal"]

# ============================================
# Medical Plausibility
# ============================================
[guardrails.plausibility]
enabled = true
# Maximum number of comorbidities by age group
max_comorbidities_young = 4      # age < 30
max_comorbidities_middle = 6     # age 30-65
max_comorbidities_elderly = 10   # age > 65
# Check gender-specific conditions
check_gender_conditions = true
# Check medication-condition requirements
check_medication_requirements = true

# ============================================
# Distribution Check
# ============================================
[guardrails.distribution]
enabled = true
# Tolerance for rate deviation (e.g., 0.05 = ±5%)
tolerance = 0.05
# Check frequency: every N batches
check_frequency_batches = 10

# ============================================
# Uniqueness Check
# ============================================
[guardrails.uniqueness]
enabled = true
# Check for duplicate IDs across batches
cross_batch_check = true
```

---

## Data Structures

### Patient Domain

```rust
// src/domain/patient.rs - Additions

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PatientName {
    pub first_name: String,
    pub last_name: String,
    pub middle_initial: Option<char>,
    pub name_suffix: Option<String>,
}

impl PatientName {
    pub fn new(first: String, last: String) -> Self {
        Self { first_name: first, last_name: last, middle_initial: None, name_suffix: None }
    }

    pub fn full(&self) -> String {
        match (&self.middle_initial, &self.name_suffix) {
            (Some(mi), Some(suffix)) => format!("{} {}. {}, {}", self.first_name, mi, self.last_name, suffix),
            (Some(mi), None) => format!("{} {}. {}", self.first_name, mi, self.last_name),
            (None, Some(suffix)) => format!("{} {}, {}", self.first_name, self.last_name, suffix),
            (None, None) => format!("{} {}", self.first_name, self.last_name),
        }
    }

    pub fn sort_key(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }

    pub fn initials(&self) -> String {
        let mut result = format!("{}. {}", self.first_name.chars().next().unwrap(), self.last_name);
        if let Some(mi) = self.middle_initial {
            result.insert(2, mi);
            result.insert(3, '.');
        }
        result
    }
}

// Updated PatientProfile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientProfile {
    pub patient_id: String,
    pub name: PatientName,  // NEW
    pub age: u8,
    pub gender: Gender,
    pub region: String,
    pub risk_bucket: RiskBucket,
}
```

### Guardrail Domain

```rust
// src/domain/guardrail.rs - NEW FILE

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ViolationType {
    PiiScan { pattern_type: String, position: usize },
    NameSafety { blocked_name: String },
    ContentPolicy { category: String, term: String, context: String },
    Plausibility { rule: String, details: String },
    Distribution { metric: String, expected: f64, actual: f64 },
    Uniqueness { duplicate_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub patient_id: String,
    pub timestamp: DateTime<Utc>,
}

impl Violation {
    pub fn severity(&self) -> ViolationSeverity {
        self.severity.clone()
    }
}

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardrailReport {
    pub job_id: String,
    pub timestamp: DateTime<Utc>,
    pub summary: SummaryStats,
    pub checks: CheckResults,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryStats {
    pub total_checked: usize,
    pub passed: usize,
    pub flagged: usize,
    pub rejected: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResults {
    pub pii_scan: PiiScanResult,
    pub name_safety: NameSafetyResult,
    pub content_policy: ContentPolicyResult,
    pub plausibility: PlausibilityResult,
    pub distribution: DistributionResult,
    pub uniqueness: UniquenessResult,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_check_time_ms: u64,
    pub avg_time_per_record_ms: f64,
}
```

---

## Implementation Plan

### Phase 1: Patient Names (1-2 hours)

| Task | File | Effort |
|------|------|--------|
| Create name data tables | `src/data/names.rs` | 30 min |
| Add PatientName struct | `src/domain/patient.rs` | 15 min |
| Implement generate_name() | `src/generation.rs` | 30 min |
| Update clinical note template | `src/generation.rs` (note generation) | 15 min |
| Add names config section | `src/config.rs` | 10 min |
| Update tests | `tests/` | 20 min |

### Phase 2: Guardrails Core (3-4 hours)

| Task | File | Effort |
|------|------|--------|
| Create guardrail domain | `src/domain/guardrail.rs` | 30 min |
| Implement PII scanner | `src/guardrails/pii.rs` | 45 min |
| Implement content policy checker | `src/guardrails/content.rs` | 30 min |
| Implement plausibility checker | `src/guardrails/plausibility.rs` | 45 min |
| Implement distribution checker | `src/guardrails/distribution.rs` | 30 min |
| Implement uniqueness checker | `src/guardrails/uniqueness.rs` | 15 min |
| Add guardrails config | `src/config.rs` | 30 min |

### Phase 3: Guardrail Actor (1-2 hours)

| Task | File | Effort |
|------|------|--------|
| Create GuardrailActor | `src/actors/guardrail.rs` | 60 min |
| Add guardrail messages | `src/actors/messages.rs` | 15 min |
| Integrate into pipeline | `src/actors/orchestrator.rs` | 30 min |
| Implement report writer | `src/output/guardrail_report.rs` | 30 min |

### Phase 4: Testing & Documentation (1-2 hours)

| Task | Description | Effort |
|------|-------------|--------|
| Unit tests for name generation | Verify determinism, gender/regional awareness | 30 min |
| Unit tests for each guardrail | Mock violations, verify detection | 45 min |
| Integration test | Full pipeline with guardrails enabled | 30 min |
| Update README.md | Document new features | 30 min |

**Total Effort:** 6-10 hours

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_generation_deterministic() {
        let seed = 42;
        let mut rng1 = ChaCha8Rng::seed_from_u64(seed);
        let mut rng2 = ChaCha8Rng::seed_from_u64(seed);

        let name1 = generate_name(&mut rng1, &Gender::Female, "Northeast");
        let name2 = generate_name(&mut rng2, &Gender::Female, "Northeast");

        assert_eq!(name1, name2);
    }

    #[test]
    fn test_pii_detection() {
        let text = "Patient SSN is 123-45-6789 and phone is 555-123-4567";
        let (redacted, violations) = scan_and_redact_pii(text);

        assert_eq!(violations.len(), 2);
        assert!(redacted.contains("[REDACTED_SSN]"));
        assert!(redacted.contains("[REDACTED_PHONE]"));
    }

    #[test]
    fn test_blocked_name_detection() {
        let blocked = PatientName::new("Donald".to_string(), "Trump".to_string());
        let violations = check_name_safety(&blocked);

        assert!(!violations.is_empty());
    }

    #[test]
    fn test_plausibility_check() {
        let record = create_test_record_with(
            age = 25,
            comorbidities = vec!["diabetes", "hypertension", "asthma",
                                 "copd", "obesity", "ckd", "cad"],  // 7 conditions
        );

        let violations = check_medical_plausibility(&record);

        assert!(violations.iter().any(|v| matches!(v, PlausibilityViolation::UnlikelyComorbidityCount { .. })));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_pipeline_with_guardrails() {
    let config = TestConfig::with_guardrails_enabled();

    let result = run_generation(&config).await;

    assert!(result.guardrail_report.is_some());
    let report = result.guardrail_report.unwrap();

    assert_eq!(report.summary.total_checked, 1000);
    assert_eq!(report.summary.rejected, 0);  // Should not reject valid data

    // Verify guardrail report file exists
    assert!(Path::new(&format!("{}/guardrail_report.json", config.output_dir)).exists());
}
```

### Property-Based Tests

```rust
#[quickcheck_macros::quickcheck]
fn test_distribution_within_tolerance(seed: u64) -> bool {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = default_config();
    let tolerance = 0.05;

    let records = generate_test_batch(&mut rng, 1000, &config);
    let violations = check_distribution(&records, &config.conditions, tolerance);

    // All deviations should be within tolerance
    violations.iter().all(|v| v.deviation <= tolerance)
}
```

---

## Performance Considerations

### Expected Impact

| Component | Overhead | Justification |
|-----------|----------|---------------|
| Name generation | +5% per record | Simple array lookup, O(1) |
| PII scanning | +10% per note | Regex on short strings (~500 chars) |
| Content policy check | +5% per note | Simple substring search |
| Plausibility check | +2% per record | Field comparisons, no I/O |
| Distribution check | +1% overall | Batch-level, amortized |
| **Total** | **~15-20%** | Acceptable for safety guarantee |

### Optimization Strategies

1. **Compile regex patterns once** — Lazy_static or once_cell
2. **Parallel guardrail checks** — Each check is independent
3. **Batch PII scanning** — Scan all notes in batch, single pass
4. **Early exit** — Skip plausibility if earlier checks failed

### Benchmarks

```
Before (100k patients): 11.9s
After (100k patients with all guardrails): ~14s (estimated)
Throughput: 9,200 → 7,100 records/sec
```

---

## Security and Privacy Considerations

### Data Privacy

1. **No real PII stored** — All names are synthetic
2. **No real data used for training** — Name data from public census
3. **Reproducibility** — Same seed produces same data, enabling auditing

### Audit Trail

- Every guardrail violation is logged with:
  - Patient ID (sanitized)
  - Violation type and severity
  - Timestamp
  - Context (e.g., offending text snippet, redacted)

### Compliance Demonstration

The guardrail report serves as evidence of:
- PII scanning and redaction
- Content policy enforcement
- Data quality validation
- Statistical correctness

This maps to healthcare AI compliance requirements (FDA AI/ML guidance, HIPAA, etc.).

---

## Future Enhancements

### Near-Term (Potential Extensions)

| Feature | Description | Effort |
|---------|-------------|--------|
| International names | Add non-US name pools (Asian, Hispanic, etc.) | 2-3 hrs |
| Fuzzy matching | Detect near-matches for blocked names | 2 hrs |
| Configurable blocked lists | Allow users to supply custom blocked names | 1 hr |
| Export guardrail rules | Serialize rules for audit/review | 1 hr |

### Long-Term (Research Directions)

| Feature | Description |
|---------|-------------|
| LLM-based validation | Use small local models for semantic content checks |
| Differential privacy | Add calibrated noise for privacy guarantees |
| Federated learning support | Enable distributed generation with guardrails |

---

## Appendix: RAG Use Case Demonstration

This feature set enables several compelling RAG demonstrations:

### Multi-Hop RAG Example

```
User Query: "What is the average age of female patients named Garcia
            who had a reaction to DrugX?"

Execution:
1. Hop 1: Find patients with last_name = "Garcia"
2. Hop 2: Filter to gender = "Female"
3. Hop 3: Filter to reaction_medication = "DrugX"
4. Hop 4: Calculate average age

Guardrails Applied:
- Name safety: Verified "Garcia" not on blocked list
- PII scan: Clinical notes redacted before indexing
- Content policy: Flagged any self-harm references in notes
- Plausibility: Verified age-reaction combinations valid
```

### Recursive RAG Example

```
User Query: "Summarize the clinical presentation of DrugX reactions
            in patients over 65, grouped by region."

Execution:
1. Retrieve: All patients with DrugX reactions, age > 65
2. Group By: Region
3. For Each Region:
   a. Retrieve clinical notes for those patients
   b. Extract symptoms/reaction descriptions
   c. Summarize (passing redacted notes to LLM)

Guardrails Applied:
- Each retrieval pass validates data provenance
- LLM only receives redacted clinical notes
- Summary generation logged with guardrail metadata
```

### Security Demonstration

```
Adversarial Input: "Find patients with SSN 123-45-6789"

Guardrail Response:
1. PII scanner detects SSN pattern in query
2. Query rejected before execution
3. Security event logged
4. User notified: "Query contains potential PII and was blocked"

→ Demonstrates defense-in-depth for RAG systems
```

---

## Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-05-13 | Initial design document |

---

## Approval

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Technical Lead | | | |
| Product Owner | | | |
| Security Review | | | |
