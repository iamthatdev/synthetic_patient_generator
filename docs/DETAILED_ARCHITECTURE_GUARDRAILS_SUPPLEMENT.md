# Detailed Architecture — Guardrails and Names Supplement

This supplement covers the new guardrail system, patient name generation, and pipeline changes added after the initial architecture document. It should be read alongside [DETAILED_ARCHITECTURE.md](DETAILED_ARCHITECTURE.md).

---

## Updated Pipeline (8 actors)

The patient pipeline now has 8 actors instead of 7. The **GuardrailActor** sits between ClinicalNoteActor and ChunkingActor:

```
OrchestratorActor
  │
  ▼
ProfileActor → ConditionActor → MedicationActor → ReactionActor
                                                       │
                                                       ▼
                                      ClinicalNoteActor → GuardrailActor → ChunkingActor
                                                                               │
                                                                               ▼
                                                                        PatientWriterActor
                                                                               │
                                                                               ▼
                                                                        OrchestratorActor
```

The actors are now spawned in this reverse order:

```
Spawn order:                Message flow:

1. PatientWriterActor  ←──  8. ChunkingActor
2. ChunkingActor       ←──  7. GuardrailActor
3. GuardrailActor      ←──  6. ClinicalNoteActor
4. ClinicalNoteActor   ←──  5. ReactionActor
5. ReactionActor       ←──  4. MedicationActor
6. MedicationActor     ←──  3. ConditionActor
7. ConditionActor      ←──  2. ProfileActor
8. ProfileActor        ←──  (dispatched by OrchestratorActor)
```

---

## Updated Batch Lifecycle

```
Orchestrator → ProfileActor:     GeneratePatientBatch {batch_id=0, ...}
ProfileActor → ConditionActor:   ProfileBatchCreated {batch_id=0, ...}
ConditionActor → MedicationActor: ConditionBatchAssigned {batch_id=0, ...}
MedicationActor → ReactionActor:  MedicationBatchAssigned {batch_id=0, ...}
ReactionActor → NoteActor:       ReactionBatchSimulated {batch_id=0, ...}
NoteActor → GuardrailActor:      ClinicalNoteBatchGenerated {batch_id=0, ...}
GuardrailActor → ChunkingActor:  ClinicalNoteBatchGenerated {batch_id=0, records=[...passed...]}
ChunkingActor → WriterActor:     ChunkBatchCreated {batch_id=0, ...}
WriterActor → Orchestrator:      BatchWritten {batch_id=0, ...}
```

The GuardrailActor receives and forwards `ClinicalNoteBatchGenerated` — it acts as a transparent filter. Records that fail error-level checks are removed; all others pass through unchanged.

---

## New Source Files

### `src/lib.rs` — Library Root

Re-exports all modules so integration tests (`tests/`) can import `synthetic_patient_data::*`.

### `src/data/mod.rs` + `src/data/names.rs` — Name Data Module

```
data/
├── mod.rs              (pub mod names)
└── names.rs            (~200 lines of name tables + generation)
```

`names.rs` provides:

- **`FIRST_NAMES_FEMALE`** — ~100 census-ranked female first names
- **`FIRST_NAMES_MALE`** — ~100 census-ranked male first names
- **`LAST_NAMES`** — ~200 census-ranked surnames (not currently used directly; regional pools are used instead)
- **`REGIONAL_POOLS`** — 5 region-specific surname pools with demographic bias:

| Region | Bias |
|---|---|
| Northeast | Higher Irish/Italian (Sullivan, O'Brien, Romano) |
| Southeast | Higher Scottish/Irish (Campbell, MacDonald, Fitzpatrick) |
| Midwest | Higher German/Scandinavian (Schmidt, Olson, Larson) |
| Southwest | Higher Hispanic (Garcia, Rodriguez, Martinez) |
| West | Diverse (Lee, Nguyen, Kim, Patel) |

- **`generate_name(rng, gender, region)`** — deterministic name generation

Algorithm:
1. Select first name pool by gender → `first_pool[rng.gen_range(0..len)]`
2. Select last name pool by region → `REGIONAL_POOLS.get_pool_for_region(region)`
3. Pick random last name from regional pool
4. 10% chance: add middle initial (A–Z)
5. 2% chance: add suffix (Jr., Sr., II, III, IV)
6. Return `PatientName`

---

### `src/domain/guardrail.rs` — Guardrail Domain Types

All guardrail-related types:

| Type | Purpose |
|---|---|
| `ViolationSeverity` | Warning or Error |
| `ViolationType` | 6 variants: PiiScan, NameSafety, ContentPolicy, Plausibility, Distribution, Uniqueness |
| `Violation` | Single violation with type, patient_id, timestamp |
| `ViolationBatch` | Collection of warnings, errors, and rejected IDs from a batch |
| `SummaryStats` | total_checked, passed, flagged, rejected |
| `PiiScanResult` | PII check results with violation list |
| `NameSafetyResult` | Blocked name detection results |
| `ContentPolicyResult` | Content violations by category with examples |
| `PlausibilityResult` | Plausibility violations by type |
| `DistributionResult` | Distribution deviations |
| `UniquenessResult` | Duplicate ID list |
| `CheckResults` | All 6 check results combined |
| `GuardrailReport` | Full report: job_id, timestamp, summary, checks, performance_metrics |
| `PerformanceMetrics` | total_check_time_ms, avg_time_per_record_ms |

---

### `src/guardrails/` — Guardrail Implementation Modules

```
guardrails/
├── mod.rs              (pub mod exports)
├── pii.rs              (PII detection and redaction)
├── content.rs          (Content policy checking)
├── plausibility.rs     (Medical plausibility rules)
├── distribution.rs     (Statistical distribution checks)
└── uniqueness.rs       (Uniqueness validation)
```

Each module is a pure function with no actor dependency. Summary:

#### `pii.rs` — PII Detection

Scans for SSN, phone, email, ZIP+4, and credit card patterns using compiled regex. Returns redacted text and violations. Patterns cached via `OnceLock`. Severity: **Error**.

| Pattern | Redaction |
|---|---|
| SSN (xxx-xx-xxxx) | `[REDACTED_SSN]` |
| Phone (xxx-xxx-xxxx) | `[REDACTED_PHONE]` |
| Email (x@x.xx) | `[REDACTED_EMAIL]` |
| ZIP+4 (xxxxx-xxxx) | `[REDACTED_ZIP]` |
| Credit card (xxxx-xxxx-xxxx-xxxx) | `[REDACTED_CARD]` |

#### `content.rs` — Content Policy

Three dangerous term categories: self_harm, violence, substance_abuse. Extracts 20-char context windows. `DangerousTermsConfig` has a `Default` with standard lists. Severity: **Warning**.

#### `plausibility.rs` — Medical Plausibility

Four rules:

| Rule | Check | Threshold |
|---|---|---|
| too_many_comorbidities | Count vs. age | <30: 4, <65: 6, 65+: 10 |
| gender_condition_mismatch | Male↔ovarian, Female↔prostate | Exact |
| medication_without_condition | Metformin without diabetes | Case-insensitive |
| age_inappropriate_medication | Lisinopril under 18 | Exact |

Severity: **Warning**.

#### `distribution.rs` — Distribution Check

Compares condition rates in a batch against configured probabilities ±tolerance. Uses its own `ConditionsConfig` with `From<&crate::config::ConditionsConfig>`. Provides `PatientRecordExt` trait for `has_condition()`. Severity: **Warning**.

#### `uniqueness.rs` — Uniqueness Check

HashSet-based duplicate detection. Severity: **Error**.

---

### `src/actors/guardrail.rs` — GuardrailActor

Receives `ClinicalNoteBatchGenerated`, runs 5 checks per batch:

**Per-record:** PII scan, content policy, medical plausibility
**Per-batch:** Distribution check, uniqueness check

Configuration:
```rust
pub struct GuardrailConfig {
    pub pii_check_enabled: bool,           // default: true
    pub content_policy_enabled: bool,       // default: true
    pub plausibility_check_enabled: bool,   // default: true
    pub distribution_check_enabled: bool,   // default: true
    pub uniqueness_check_enabled: bool,     // default: true
    pub distribution_tolerance: f64,        // default: 0.05
    pub fail_on_error: bool,                // default: false
}
```

Processing flow (pseudocode):
```
for record in batch:
    if pii_check_enabled: scan clinical notes for PII
    if content_policy_enabled: check clinical notes for dangerous terms
    if plausibility_check_enabled: validate medical plausibility
    if fail_on_error AND has_errors: reject(record)
    else: passed_records.push(record)

if distribution_check_enabled: check condition rates
if uniqueness_check_enabled: check for duplicates, remove them

downstream.cast(ClinicalNoteBatchGenerated { records: passed_records })
```

---

### `src/output/guardrail_report.rs` — Guardrail Report Writer

Async function `write_guardrail_report(report, output_dir)` that writes `guardrail_report.json` with pretty-printed JSON.

---

## Updated Domain Types

### `PatientName` (new in `src/domain/patient.rs`)

```rust
pub struct PatientName {
    pub first_name: String,
    pub last_name: String,
    pub middle_initial: Option<char>,
    pub name_suffix: Option<String>,
}
```

Methods: `new()`, `with_middle()`, `with_suffix()`, `full()` (formats as "First M. Last, Suffix"), `sort_key()` ("Last, First"), `initials()` ("F. Last").

Derives `Clone`, `Debug`, `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash`.

### Updated `PatientProfile`

Now includes `name: PatientName` field. Created inside `generate_profile()`.

### Updated `PatientRecord`

Now includes `name: PatientName` field. Flows from the note actor through guardrail to chunking/writer.

---

## Updated Dependencies

Two new crates added to `Cargo.toml`:

| Crate | Version | Purpose |
|---|---|---|
| `chrono` | 0.4 (with serde feature) | Timestamps on guardrail violations |
| `regex` | 1 | PII pattern detection |

---

## Updated Configuration

`JobConfig` now includes a `guardrails: GuardrailsConfig` field with full serde defaults:

```toml
[guardrails]
enabled = true
fail_on_error = false
generate_report = true

[guardrails.pii]
enabled = true
detect_ssn = true
detect_phone = true
detect_email = true

[guardrails.content_policy]
enabled = true
self_harm_terms = ["suicide", "self-harm"]
violence_terms = ["homicide", "assault"]

[guardrails.plausibility]
enabled = true
max_comorbidities_young = 4
max_comorbidities_elderly = 10
check_gender_conditions = true

[guardrails.distribution]
enabled = true
tolerance = 0.05

[guardrails.uniqueness]
enabled = true
```

---

## Updated Determinism

Name generation is part of the same deterministic RNG tree as the rest of the patient profile. The `generate_name()` function is called inside `generate_profile()`, using the same `ChaCha8Rng` instance. Same seed + same config = identical names.

```
root_seed = 42
├── batch_rng(42 ^ 0)
│   ├── patient_rng(0) → P000000000
│   │   └── name: FIRST_NAMES_MALE[idx], REGIONAL_POOLS.Northeast[idx]
│   └── patient_rng(1) → P000000001
│       └── name: FIRST_NAMES_FEMALE[idx], REGIONAL_POOLS.West[idx]
└── ...
```

---

## Test Files

### `tests/name_generation_tests.rs`

7 tests covering `PatientName` formatting and clinical note name usage:

- `test_patient_name_full`
- `test_patient_name_with_middle`
- `test_patient_name_with_suffix`
- `test_patient_name_full_with_middle_and_suffix`
- `test_patient_name_sort_key`
- `test_patient_name_initials`
- `test_clinical_note_includes_name`

### `tests/guardrail_integration_tests.rs`

1 integration test (`test_full_guardrail_pipeline`) that runs all 5 guardrail checks on test records and verifies PII detection, plausibility violations, and uniqueness.

### Unit tests (inside `src/`)

21 unit tests across the guardrail modules:

- `pii.rs`: 6 tests (detect/redact SSN, email, no PII, multiple types)
- `content.rs`: 4 tests (detect self-harm, no violations, context extraction, multiple)
- `plausibility.rs`: 4 tests (too many comorbidities, gender mismatch, medication without condition, valid record)
- `distribution.rs`: 2 tests (within/outside tolerance)
- `uniqueness.rs`: 3 tests (no duplicates, with duplicates, multiple duplicates)
- `names.rs`: 2 tests (deterministic generation, regional pool selection)
