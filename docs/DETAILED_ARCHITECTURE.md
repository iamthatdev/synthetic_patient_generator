# Detailed Architecture

> **Note:** This document describes the original 7-actor pipeline. For the updated 8-actor pipeline with guardrails and patient name generation, see [DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md](DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md). Key additions: `PatientName` struct, `GuardrailActor` (5 validation checks), `GuardrailsConfig`, `src/data/` name module, `src/guardrails/` modules, and `chrono`/`regex` dependencies.

This document provides a complete technical reference for the Synthetic Patient Data Generator: every package, every file, every actor, and the full data flow.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Package Dependencies](#2-package-dependencies)
3. [Source File Reference](#3-source-file-reference)
4. [Data Model](#4-data-model)
5. [Actor Pipeline](#5-actor-pipeline)
6. [Message Flow](#6-message-flow)
7. [Determinism and Reproducibility](#7-determinism-and-reproducibility)
8. [Configuration](#8-configuration)

---

## 1. System Overview

The generator is a single Rust binary that runs a supervised actor pipeline. There is no network layer, no database, and no external service dependency. The entire system is:

- **Self-contained** — one binary, one command
- **Deterministic** — same seed + config = identical output
- **Actor-based** — each pipeline stage is an independent Ractor actor
- **Batch-oriented** — data flows in batches of 1,000 records (configurable)
- **Guardrailed** — five validation checks run inline on every batch before data is written
- **Two-phase** — patient generation runs first, then eval generation uses the patient data as input

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLI (clap)                                 │
│  $ synthetic_patient_data generate --patients 100k --evals 10k          │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │ parses args, loads config
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        main.rs + generate()                             │
│  Spawns OrchestratorActor with JobConfig, awaits completion             │
└──────────────────────────────┬──────────────────────────────────────────┘
                               │
          ┌────────────────────┴────────────────────┐
          ▼                                         ▼
┌───────────────────────┐               ┌────────────────────────┐
│   PHASE 1: Patient    │               │  PHASE 2: Eval         │
│   Generation Pipeline │──── done ────▶│  Generation Pipeline   │
│   (8 actors)          │               │  (3 actors)            │
└───────────────────────┘               └────────────────────────┘
          │                                         │
          ▼                                         ▼
  patients.jsonl                            evals.jsonl
  clinical_notes.jsonl                      ragas_dataset.jsonl
  chunks.jsonl
  summary.json
```

---

## 2. Package Dependencies

```toml
[dependencies]
ractor = { version = "0.15", features = ["async-trait"] }  # Actor framework
tokio = { version = "1", features = ["full"] }              # Async runtime
serde = { version = "1", features = ["derive"] }            # Serialization traits
serde_json = "1"                                             # JSON serialization
toml = "0.8"                                                 # Config file parsing
clap = { version = "4", features = ["derive"] }              # CLI argument parsing
rand = "0.8"                                                 # RNG traits
rand_chacha = "0.3"                                          # ChaCha8 deterministic RNG
tracing = "0.1"                                              # Structured logging
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
thiserror = "2"                                              # Error type derive
uuid = { version = "1", features = ["v4"] }                  # Unique IDs (future use)
csv = "1"                                                    # CSV export (future use)
async-trait = "0.1"                                          # Async trait support for ractor
chrono = { version = "0.4", features = ["serde"] }           # Timestamps for guardrail violations
regex = "1"                                                  # PII pattern detection
```

Each dependency serves exactly one purpose:

| Crate | Role |
|---|---|
| **ractor** | The actor framework. Provides `Actor` trait, `ActorRef`, `spawn`, `spawn_linked`, supervision, and typed message passing. Built on Tokio. |
| **tokio** | Async runtime. Every actor runs as a Tokio task. File I/O is async. |
| **serde / serde_json** | All data model types derive `Serialize` / `Deserialize`. JSONL output is one `serde_json::to_string` per record. |
| **toml** | Config file is parsed from TOML into `JobConfig` via serde. |
| **clap** | CLI subcommands (`generate`, `resume`, `validate`, `summarize`) with typed args. |
| **rand / rand_chacha** | `ChaCha8Rng` provides deterministic, seeded randomness. No `thread_rng()` anywhere. |
| **tracing** | All log output goes through `tracing::info!` / `error!` with structured fields. |
| **tracing-subscriber** | Configures log output format (JSON or pretty) and level filtering. |
| **thiserror** | `AppError` enum with `#[from]` conversions from `io::Error` and `serde_json::Error`. |
| **async-trait** | Required by ractor 0.15 for the `Actor` trait's async methods. |
| **chrono** | Timestamps on guardrail violations and reports. RFC 3339 format in JSON output. |
| **regex** | Compiled regex patterns for PII detection. Cached via `OnceLock` for performance. |

---

## 3. Source File Reference

### `src/main.rs` — Entry Point

Parses CLI args via `clap`, loads config, creates a Tokio runtime, and spawns the `OrchestratorActor`. Declares all top-level modules: `actors`, `cli`, `config`, `data`, `domain`, `errors`, `eval_generation`, `generation`, `guardrails`, `output`, `rng`.

### `src/lib.rs` — Library Root

Re-exports all modules for integration tests. Without this file, `tests/` cannot import `synthetic_patient_data::*`.

---

### `src/cli.rs` — CLI Definitions

Defines four subcommands via `clap::Parser`:

| Subcommand | Status | Purpose |
|---|---|---|
| `generate` | Implemented | Run the full pipeline |
| `resume` | Stub | Resume from checkpoint |
| `validate` | Stub | Validate output files |
| `summarize` | Stub | Print dataset statistics |

---

### `src/config.rs` — Configuration

The `JobConfig` struct and all nested config types:

```
JobConfig
├── batch_size: usize                (default: 1000)
├── patient_count: u64               (default: 10000)
├── eval_count: u64                  (default: 1000)
├── seed: u64                        (default: 42)
├── output_dir: PathBuf              (default: ./data)
├── formats: Vec<String>             (default: ["jsonl"])
├── actors: ActorConfig              (worker counts, buffer sizes)
├── demographics: DemographicsConfig (age range, gender probability)
├── conditions: ConditionsConfig     (per-condition probabilities)
├── medications: MedicationsConfig   (per-medication exposure rates)
├── reactions: ReactionsConfig       (per-drug reaction rules)
├── evals: EvalsConfig               (query type mix, difficulty)
├── observability: ObservabilityConfig (log level, metrics)
├── fault_tolerance: FaultToleranceConfig (retries, checkpoint path)
└── guardrails: GuardrailsConfig     (guardrail toggle, per-check config)
```

Every field has a serde default, so a TOML file can override any subset. CLI flags (`--patients`, `--seed`, etc.) override config file values.

---

### `src/rng.rs` — Deterministic RNG

Two functions that create seeded `ChaCha8Rng` instances:

```rust
fn batch_rng(root_seed: u64, batch_id: u64) -> ChaCha8Rng
    // seed = root_seed ^ batch_id

fn patient_rng(batch: &mut ChaCha8Rng, patient_index: u64) -> ChaCha8Rng
    // seed = batch.next_u64() ^ patient_index
```

This creates a two-level deterministic RNG tree:

```
root_seed
├── batch_rng(seed=42 ^ batch_id=0)
│   ├── patient_rng(index=0)  → patient P000000000
│   ├── patient_rng(index=1)  → patient P000000001
│   └── ...
├── batch_rng(seed=42 ^ batch_id=1)
│   ├── patient_rng(index=0)  → patient P000001000
│   └── ...
└── ...
```

---

### `src/generation.rs` — Patient Generation Logic

Pure functions with no actor dependency. Each function takes a `&mut ChaCha8Rng` and config, returns a transformed data type:

| Function | Input → Output |
|---|---|
| `generate_profile` | `(rng, global_index, demographics_config) → PatientProfile` |
| `assign_conditions` | `(rng, profile, conditions_config) → PatientWithConditions` |
| `assign_medications` | `(rng, patient, medications_config) → PatientWithMedications` |
| `simulate_reaction` | `(rng, patient, reactions_config) → PatientRecordDraft` |
| `generate_clinical_note_text` | `(draft) → String` |

`generate_profile` now calls `crate::data::names::generate_name(rng, &gender, &region)` to produce a deterministic `PatientName`. Clinical notes use `draft.profile.name.full()` instead of the patient ID, producing natural text like "Patient Mary Smith is a 45-year-old female..."

Reaction simulation rules:
- DrugX: 20% base reaction probability, +5% if diabetic, +10% severe probability if age > 65
- DrugY: 8% base reaction probability, +3% if diabetic, +5% severe probability if age > 65

---

### `src/data/` — Name Data Module

```
data/
├── mod.rs              (pub mod names)
└── names.rs            (~200 lines of name tables + generation)
```

`names.rs` provides census-based name tables (~100 female first names, ~100 male first names, ~200 surnames) and 5 regional surname pools with demographic bias. The `generate_name(rng, gender, region)` function produces deterministic `PatientName` values with optional middle initials (10%) and suffixes (2%).

---

### `src/eval_generation.rs` — Eval Query Generation

`EvalContext` wraps `Vec<PatientRecord>` and provides accessor methods (`drug_x_reactors()`, `drug_x_exposed()`, etc.).

`generate_eval_record()` picks a query type based on config probabilities, then dispatches to one of 7 generator functions:

| Function | Query Type | Example |
|---|---|---|
| `generate_lookup` | Lookup | "Which patients had a reaction to DrugX?" |
| `generate_filtered_lookup` | FilteredLookup | "Which female patients over 65 had a DrugX reaction?" |
| `generate_aggregation` | Aggregation | "What percentage of DrugX patients had an adverse reaction?" |
| `generate_multihop` | MultiHop | "Among diabetic patients prescribed DrugX, how many had severe rash?" |
| `generate_negative` | Negative | "Which patients had an allergy to DrugQ?" |
| `generate_comparative` | Comparative | "Was DrugX or DrugY associated with more adverse reactions?" |
| `generate_evidence` | EvidenceRetrieval | "Find clinical notes supporting DrugX allergy in patient P000123." |

Each function scans the `EvalContext` to find matching patients, builds ground truth, and collects relevant clinical note texts as contexts.

`to_ragas_record()` flattens an `EvalRecord` into the simpler `RagasRecord` format (question/answer/contexts/ground_truth).

---

### `src/domain/` — Data Model Types

```
domain/
├── mod.rs
├── patient.rs
│   ├── Gender              (Female, Male)
│   ├── Severity            (Mild, Moderate, Severe)
│   ├── RiskBucket          (Low, Medium, High)
│   ├── PatientName         (first_name, last_name, middle_initial, name_suffix)
│   ├── PatientProfile      (id, name, age, gender, region, risk_bucket)
│   ├── PatientWithConditions
│   ├── PatientWithMedications
│   ├── PatientRecordDraft  (includes reaction fields)
│   ├── PatientMetadata     (seed, batch_id)
│   └── PatientRecord       (full record with name, notes, metadata)
├── clinical_note.rs
│   ├── ClinicalNote        (note_id, patient_id, note_type, text)
│   ├── CorpusChunk         (chunk_id, patient_id, text, source, metadata)
│   └── PatientOutput       (record + chunks)
├── eval.rs
│   ├── EvalQueryType       (7 variants)
│   ├── Difficulty          (Easy, Medium, Hard)
│   ├── GroundTruthFact     (patient_id, fact)
│   ├── EvalMetadata        (seed, batch_id)
│   ├── EvalRecord          (full eval with ground truth)
│   └── RagasRecord         (RAGAS-compatible flattened format)
└── guardrail.rs
    ├── ViolationSeverity   (Warning, Error)
    ├── ViolationType       (6 variants)
    ├── Violation           (type, patient_id, timestamp)
    ├── ViolationBatch      (warnings, errors, rejected_ids)
    ├── SummaryStats, CheckResults, GuardrailReport, PerformanceMetrics
    └── Per-check result structs (PiiScanResult, ContentPolicyResult, etc.)
```

All types derive `Clone`, `Debug`, `Serialize`, `Deserialize`. `PatientName` also derives `PartialEq`, `Eq`, `Hash`.

---

### `src/guardrails/` — Guardrail Implementation Modules

```
guardrails/
├── mod.rs
├── pii.rs              (PII detection and redaction via regex)
├── content.rs          (dangerous term checking, 3 categories)
├── plausibility.rs     (medical plausibility rules, 4 rules)
├── distribution.rs     (condition rate deviation checking)
└── uniqueness.rs       (duplicate ID detection)
```

See [DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md](DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md) for detailed descriptions of each module.

---

### `src/output/` — Output Writers

```
output/
├── mod.rs
├── jsonl.rs
│   └── JsonlWriter         (async, writes one JSON object per line)
└── guardrail_report.rs
    └── write_guardrail_report()  (async, writes GuardrailReport as JSON)
```

`JsonlWriter` wraps a Tokio `File` with `serde_json::to_string` per record. It creates parent directories, writes individual records or batches, flushes on demand, and tracks a count of records written.

---

### `src/actors/` — The Actor Pipeline

This is the core of the system. Each file implements one or more actors.

#### `src/actors/messages.rs` — PipelineMsg

The single message enum used by the patient pipeline:

```rust
enum PipelineMsg {
    GeneratePatientBatch { job_id, batch_id, start_index, size, seed },
    ProfileBatchCreated { job_id, batch_id, profiles },
    ConditionBatchAssigned { job_id, batch_id, patients },
    MedicationBatchAssigned { job_id, batch_id, patients },
    ReactionBatchSimulated { job_id, batch_id, patients },
    ClinicalNoteBatchGenerated { job_id, batch_id, records },
    ChunkBatchCreated { job_id, batch_id, output },
    BatchWritten { job_id, batch_id, records_written },
    BatchFailed { job_id, batch_id, actor, reason },
    Shutdown,
}
```

Every actor in the patient pipeline uses `PipelineMsg` as its message type. This allows any actor to hold an `ActorRef<PipelineMsg>` to any other actor.

---

#### Actor Wiring

The actors are spawned in **reverse order** during `OrchestratorActor::pre_start`. The current pipeline has 8 actors (GuardrailActor sits between ClinicalNoteActor and ChunkingActor):

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

Each actor receives the next actor's `ActorRef` as a startup argument and stores it in its state. This creates a **forward-chain**: when an actor finishes processing a batch, it forwards the result to the next actor.

---

#### `src/actors/orchestrator.rs` — OrchestratorActor

The top-level coordinator. Responsibilities:

1. **Spawns all pipeline actors** (8 for patient pipeline, including GuardrailActor)
2. **Dispatches all patient batches** in `pre_start`
3. **Tracks progress** via `HashSet<u64>` of completed batch IDs
4. **Launches eval pipeline** after all patient batches complete
5. **Writes `summary.json`** after eval pipeline finishes

State:
```rust
struct OrchestratorState {
    job_id: String,
    config: JobConfig,
    total_batches: usize,
    completed_batches: HashSet<u64>,
    failed_batches: Vec<(u64, String)>,
    started_at: Instant,
    eval_launched: bool,
    profile_actor: ActorRef<PipelineMsg>,
    writer_actor: ActorRef<PipelineMsg>,
}
```

The orchestrator is the **only actor that knows about both pipelines**. After patient generation, it:
1. Shuts down the patient pipeline actors
2. Reads `patients.jsonl` from disk into `Vec<PatientRecord>`
3. Wraps them in `EvalContext`
4. Spawns `EvalOrchestratorActor` with the context
5. Awaits the eval pipeline's `JoinHandle`

---

#### `src/actors/profile.rs` — ProfileActor

Receives `GeneratePatientBatch`, calls `generation::generate_profile()` for each patient in the batch (which now includes name generation), sends `ProfileBatchCreated` downstream.

State: downstream `ActorRef`, `DemographicsConfig`.

---

#### `src/actors/condition.rs` — ConditionActor

Receives `ProfileBatchCreated`, calls `generation::assign_conditions()` for each profile, sends `ConditionBatchAssigned` downstream.

State: downstream `ActorRef`, `ConditionsConfig`.

---

#### `src/actors/medication.rs` — MedicationActor

Receives `ConditionBatchAssigned`, calls `generation::assign_medications()` for each patient, sends `MedicationBatchAssigned` downstream.

State: downstream `ActorRef`, `MedicationsConfig`.

---

#### `src/actors/reaction.rs` — ReactionActor

Receives `MedicationBatchAssigned`, calls `generation::simulate_reaction()` for each patient, sends `ReactionBatchSimulated` downstream.

State: downstream `ActorRef`, `ReactionsConfig`.

---

#### `src/actors/note.rs` — ClinicalNoteActor

Receives `ReactionBatchSimulated`, calls `generation::generate_clinical_note_text()` for each draft (uses `draft.profile.name.full()` for natural patient identification), assembles full `PatientRecord` with notes, sends `ClinicalNoteBatchGenerated` downstream.

State: downstream `ActorRef`.

---

#### `src/actors/guardrail.rs` — GuardrailActor

Receives `ClinicalNoteBatchGenerated`, runs five validation checks (PII, content policy, plausibility per record; distribution and uniqueness per batch). Records with error-level violations are optionally rejected. Passed records forwarded to ChunkingActor.

State: downstream `ActorRef`, `GuardrailConfig`, `ConditionsConfig`.

See [DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md](DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md) for full details.

---

#### `src/actors/chunking.rs` — ChunkingActor

Receives `ClinicalNoteBatchGenerated`, creates a `CorpusChunk` for each clinical note (one per patient in V1), wraps into `PatientOutput`, sends `ChunkBatchCreated` downstream.

State: downstream `ActorRef`.

---

#### `src/actors/writer.rs` — PatientWriterActor

Receives `ChunkBatchCreated`, writes three JSONL files:
- `patients.jsonl` — full `PatientRecord` per line
- `clinical_notes.jsonl` — one `ClinicalNote` per line (flattened from all notes)
- `chunks.jsonl` — one `CorpusChunk` per line

Then sends `BatchWritten` to the orchestrator. On `Shutdown`, flushes all writers.

State: three `JsonlWriter` instances, orchestrator `ActorRef`, record count.

---

#### `src/actors/eval_orchestrator.rs` — Eval Pipeline (3 Actors)

This file contains three actors for the eval pipeline:

**EvalOrchestratorActor** — Spawns the other two, dispatches eval batches, tracks completion.

**EvalQueryActor** — Receives `GenerateEvalBatch`, holds the `EvalContext` (all patient records in memory), calls `eval_generation::generate_eval_record()` for each eval, sends `EvalBatchGenerated` downstream.

**EvalWriterActor** — Receives `EvalBatchGenerated`, writes both:
- `evals.jsonl` — full `EvalRecord` per line
- `ragas_dataset.jsonl` — flattened `RagasRecord` per line

Sends `EvalBatchWritten` to `EvalOrchestratorActor`.

---

## 4. Data Model

### Patient Pipeline Types

```
PatientName                        (first_name, last_name, middle_initial?, name_suffix?)
  └── PatientProfile               (+ id, age, gender, region, risk_bucket)
      → PatientWithConditions      (+ comorbidities)
        → PatientWithMedications   (+ medications)
          → PatientRecordDraft     (+ reaction fields)
            → PatientRecord        (+ name, clinical_notes, metadata)

Parallel: ClinicalNote → CorpusChunk → PatientOutput(record, chunks)
```

`PatientName` is created inside `generate_profile()` and flows through every subsequent stage.

### Eval Pipeline Types

```
EvalContext(Vec<PatientRecord>)  →  EvalRecord  →  RagasRecord
```

### Guardrail Types

```
ViolationType (6 variants) → Violation → ViolationBatch → SummaryStats
CheckResults (6 check structs) → GuardrailReport (+ job_id, summary, performance_metrics)
```

### JSON Output Shapes

Every `Serialize` type writes directly to JSONL. No transformation layer. The `serde` field names are the JSON keys.

---

## 5. Actor Pipeline

### Phase 1: Patient Generation

```
OrchestratorActor
  │ dispatches N batches of GeneratePatientBatch
  │
  ▼
ProfileActor
  │ generates demographics + patient name for batch
  │ sends ProfileBatchCreated
  ▼
ConditionActor
  │ assigns comorbidities
  │ sends ConditionBatchAssigned
  ▼
MedicationActor
  │ assigns medications
  │ sends MedicationBatchAssigned
  ▼
ReactionActor
  │ simulates adverse reactions
  │ sends ReactionBatchSimulated
  ▼
ClinicalNoteActor
  │ generates clinical note text (using patient name)
  │ sends ClinicalNoteBatchGenerated
  ▼
GuardrailActor
  │ runs 5 validation checks, rejects or passes records
  │ forwards passed records as ClinicalNoteBatchGenerated
  ▼
ChunkingActor
  │ creates corpus chunks
  │ sends ChunkBatchCreated
  ▼
PatientWriterActor
  │ writes patients.jsonl, clinical_notes.jsonl, chunks.jsonl
  │ sends BatchWritten to OrchestratorActor
  ▼
OrchestratorActor
  │ tracks completed batches
  │ when all done: loads patient data, launches eval pipeline
```

### Phase 2: Eval Generation

```
OrchestratorActor
  │ spawns EvalOrchestratorActor with EvalContext
  │
  ▼
EvalOrchestratorActor
  │ dispatches M batches of GenerateEvalBatch
  │
  ▼
EvalQueryActor
  │ holds EvalContext (all patient records)
  │ generates queries + resolves ground truth
  │ sends EvalBatchGenerated
  ▼
EvalWriterActor
  │ writes evals.jsonl, ragas_dataset.jsonl
  │ sends EvalBatchWritten to EvalOrchestratorActor
  ▼
EvalOrchestratorActor
  │ when all done: stops itself
  ▼
OrchestratorActor
  │ writes summary.json
  │ stops itself
```

---

## 6. Message Flow

### Batch Lifecycle (Patient Pipeline)

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

The GuardrailActor receives and forwards the same `ClinicalNoteBatchGenerated` message type — it acts as a transparent filter. Records that fail error-level checks are removed; all others pass through unchanged.

All batches flow through the same chain independently. Because Ractor processes one message at a time per actor, batches are processed **sequentially within each actor** but the pipeline is **naturally pipelined** — while the ReactionActor processes batch 3, the GuardrailActor can process batch 2 and the ChunkingActor can process batch 1.

### Progress Tracking

The orchestrator tracks completed batches in a `HashSet<u64>`. Every 10 batches or on completion, it logs progress with throughput. When all batches are written, it triggers eval generation.

---

## 7. Determinism and Reproducibility

The system guarantees identical output for the same seed and config through three mechanisms:

### 7.1 Seeded RNG Tree

```
root_seed = 42
├── Patient ProfileActor batch 0: batch_rng(42 ^ 0)
│   ├── patient 0: patient_rng(batch0, 0) → P000000000
│   │   └── name: FIRST_NAMES_MALE[idx], REGIONAL_POOLS.Northeast[idx]
│   └── patient 999: patient_rng(batch0, 999) → P000000999
├── Patient ConditionActor batch 0: batch_rng(0, 0)  ← different seed domain
├── Patient MedicationActor batch 0: batch_rng(1, 0)
├── Patient ReactionActor batch 0: batch_rng(2, 0)
├── Eval QueryActor batch 0: batch_rng(seed ^ 0xDEAD_BEEF_CAFE_BABE, 0)
└── ...
```

Name generation is part of `generate_profile()`, so names are derived from the same deterministic RNG as the rest of the profile. Each pipeline stage uses a different seed derivation so they don't interfere.

### 7.2 Deterministic Patient IDs

```rust
let patient_id = format!("P{:09}", global_index);
```

Patient IDs are derived from the global index, not from UUIDs or random generation.

### 7.3 Batch-Sequential Processing

Within each actor, Ractor processes messages one at a time in FIFO order. Because batches are dispatched in order (batch 0, 1, 2, ...) and each actor processes them sequentially, the output is deterministic.

---

## 8. Configuration

### Default Values

| Setting | Default |
|---|---|
| `batch_size` | 1000 |
| `patient_count` | 10000 |
| `eval_count` | 1000 |
| `seed` | 42 |
| `output_dir` | `./data` |
| `formats` | `["jsonl"]` |

### Condition Probabilities

| Condition | Default |
|---|---|
| diabetes | 12% |
| hypertension | 28% |
| asthma | 9% |
| chronic_kidney_disease | 4% |
| coronary_artery_disease | 6% |
| copd | 5% |
| obesity | 22% |

### Medication Exposure Rates

| Medication | Default |
|---|---|
| DrugX | 5% |
| DrugY | 8% |
| DrugZ | 3% |
| Aspirin | 18% |
| Metformin (if diabetic) | 70% |

### Reaction Rules

| Drug | Reaction Prob | Severe Prob | Reaction Types |
|---|---|---|---|
| DrugX | 20% | 15% | rash, hives, shortness of breath |
| DrugY | 8% | 5% | nausea, dizziness, rash |

Modifiers: diabetes increases reaction probability, age > 65 increases severe probability.

### Guardrail Configuration

| Check | Default | Severity |
|---|---|---|
| PII Detection | enabled | Error |
| Content Policy | enabled | Warning |
| Medical Plausibility | enabled | Warning |
| Distribution | enabled, tolerance 5% | Warning |
| Uniqueness | enabled | Error |

All configurable via TOML. See [DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md](DETAILED_ARCHITECTURE_GUARDRAILS_SUPPLEMENT.md) for the full `GuardrailsConfig` schema.

### Eval Query Mix

| Query Type | Allocation |
|---|---|
| Aggregation | ~15% |
| Multi-hop | ~15% |
| Negative control | ~10% |
| Comparative | ~15% |
| Evidence retrieval | ~20% |
| Filtered lookup | ~13% |
| Lookup | ~12% |

All configurable via TOML.
