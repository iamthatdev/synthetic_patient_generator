# Tightened Product + PRD: Ractor-Powered Synthetic Healthcare Data & EvalSet Generator

## 1. Executive Summary

Build a **high-throughput synthetic healthcare data generation system in Rust using Ractor**, a pure-Rust actor framework built on Tokio and inspired by Erlang `gen_server`-style actor supervision.

The system generates:

1. **Synthetic patient records**
2. **Synthetic clinical notes**
3. **Medication and adverse reaction scenarios**
4. **RAG / RAGAS-style evaluation datasets**
5. **Ground-truth answer sets**
6. **Retrieval contexts for benchmarking**

The core value of using **Ractor** is not merely parallelism. It is the ability to model the generator as a **fault-tolerant, supervised, message-driven production system** where each stage of the pipeline can scale independently.

This is ideal for generating anything from small demo datasets to very large benchmark corpora, such as:

```text
10,000 patients
100,000 patients
1,000,000+ patients
10,000+ eval questions
100,000+ eval questions
```

---

# 2. Product Name

## Working Name

**Synthetic Patient Data Generator**

## Full Name

**Synthetic Patient Data Generator: Ractor-Powered Synthetic Healthcare Data and EvalSet Generator**

## Positioning

> A high-performance Rust-based synthetic healthcare data and evaluation dataset generator for RAG, recursive retrieval, agentic QA, and healthcare AI benchmarking.

---

# 3. Product Vision

Synthetic Patient Data Generator enables developers, researchers, and AI teams to generate **reproducible, realistic, configurable synthetic healthcare datasets** and **ground-truth evaluation sets** at scale.

The system should be:

- Fast
- Reproducible
- Configurable
- Fault-tolerant
- Observable
- Easy to run from CLI
- Useful for RAG benchmarking
- Easy to explain in a technical blog or architecture demo

---

# 4. Why Ractor?

Ractor is a strong fit because the system is naturally modeled as an actor pipeline:

```text
Generate profiles → assign conditions → assign medications → simulate reactions → write records → generate evals
```

Each step can be isolated into independently scalable actors.

Ractor provides:

- Actor-based concurrency
- Supervision-style architecture
- Typed message passing
- Isolation of failures
- Natural pipeline composition
- Tokio-based async runtime
- A clean story for system design diagrams and blog explanations

The goal is not to over-engineer a simple script. The goal is to build a **miniature production-grade synthetic data factory**.

---

# 5. Problem Statement

Teams building healthcare RAG systems, clinical QA demos, recursive retrieval systems, or AI evaluation pipelines need synthetic data that is:

- Large enough for retrieval benchmarking
- Rich enough to test multi-hop questions
- Reproducible across runs
- Configurable by medical distributions
- Safe to share publicly
- Fast to generate
- Exportable into common data formats

Most quick prototypes are written as Python scripts. These are often:

- Single-threaded
- Slow at large scale
- Hard to supervise
- Hard to resume after failure
- Poorly instrumented
- Difficult to turn into a compelling systems blog

Synthetic Patient Data Generator solves this by using Rust + Ractor to create a concurrent, supervised synthetic data generation system.

---

# 6. Target Users

## 6.1 Primary User: ML / AI Engineer

### Needs

- Generate synthetic healthcare documents for RAG.
- Create eval sets with known answers.
- Benchmark retrieval quality.
- Test recursive RAG pipelines.

### Example

```bash
Synthetic Patient Data Generator generate \
  --patients 100000 \
  --evals 10000 \
  --seed 42 \
  --output ./data
```

---

## 6.2 Secondary User: AI Researcher

### Needs

- Generate controlled datasets.
- Vary disease, medication, and reaction distributions.
- Re-run experiments with the same seed.
- Compare retrieval methods fairly.

---

## 6.3 Secondary User: Technical Blogger / Educator

### Needs

- Demonstrate actor systems in Rust.
- Explain supervision and concurrency visually.
- Show real benchmark results.
- Publish reproducible examples.

---

## 6.4 Future User: Data Platform Engineer

### Needs

- Run large synthetic data jobs.
- Monitor throughput and failures.
- Export JSONL, CSV, and Parquet.
- Integrate with downstream indexing pipelines.

---

# 7. Product Goals

## 7.1 Core Goals

| Goal | Description |
|---|---|
| Generate patient data at scale | Produce 10k to 1M+ synthetic records |
| Generate eval sets | Produce RAG/RAGAS-style QA pairs with contexts |
| Ensure reproducibility | Same seed and config must produce identical output |
| Use actor-based concurrency | Pipeline must be implemented with Ractor actors |
| Support fault tolerance | Failed worker actors should be restartable |
| Support configurable distributions | Medical, demographic, medication, and reaction distributions should be TOML-driven |
| Export useful formats | JSONL first, then CSV and Parquet |
| Provide observability | Logs, counters, progress, and optional Prometheus metrics |

---

## 7.2 Non-Goals

The first version should **not** attempt to be a clinically accurate medical simulator.

Out of scope for V1:

- Real patient data
- HIPAA handling
- Clinical validation
- Claims data modeling
- Full FHIR implementation
- EHR integration
- Real drug-drug interaction modeling
- LLM-based clinical reasoning as a required dependency
- Complex longitudinal patient timelines

This is a **synthetic benchmarking generator**, not a medical-grade simulation platform.

---

# 8. Core Product Capabilities

## 8.1 Synthetic Patient Record Generation

Generate structured patient records with:

- Patient ID
- Age
- Gender
- Region
- Comorbidities
- Medications
- Allergies
- Adverse reactions
- Risk flags
- Clinical note
- Source chunks for retrieval
- Metadata for traceability

---

## 8.2 Synthetic Clinical Notes

Each patient should have one or more clinical-note-like documents.

Example:

```text
Patient P000123 is a 68-year-old female with diabetes and hypertension.
She was prescribed DrugX and aspirin. Within 24 hours of DrugX exposure,
she developed a severe rash and shortness of breath. DrugX was discontinued.
Reaction resolved after antihistamine treatment.
```

These notes become the retrieval corpus.

---

## 8.3 Medication and Reaction Simulation

The system should support rule-driven simulation.

Example:

```toml
[medications.DrugX]
exposure_probability = 0.05

[reactions.DrugX]
reaction_probability = 0.20
severe_probability = 0.15
risk_factors = ["age_over_65", "diabetes"]
```

This allows synthetic scenarios like:

- DrugX allergy
- DrugY mild intolerance
- DrugZ severe adverse event
- Elderly patients with elevated risk
- Comorbidity-based reaction probability

---

## 8.4 EvalSet Generation

Generate evaluation queries and ground-truth answers from the synthetic dataset.

Eval types:

| Eval Type | Example Query |
|---|---|
| Lookup | “Which patients had a severe reaction to DrugX?” |
| Filtered lookup | “Which female patients over 65 had a DrugX allergy?” |
| Aggregation | “What percentage of DrugX patients had severe reactions?” |
| Multi-hop | “Among diabetic patients prescribed DrugX, how many had severe rash?” |
| Negative control | “Which patients had an allergy to DrugQ?” |
| Comparative | “Was DrugX or DrugY associated with more severe reactions?” |
| Evidence retrieval | “Find clinical notes supporting DrugX allergy in patient P000123.” |

---

## 8.5 Ground Truth Generation

Each eval record should include machine-checkable ground truth.

Example:

```json
{
  "eval_id": "E000001",
  "query": "Which patients over 65 had a severe reaction to DrugX?",
  "answer": "P000123, P000451, P000982",
  "ground_truth_patient_ids": ["P000123", "P000451", "P000982"],
  "ground_truth_facts": [
    {
      "patient_id": "P000123",
      "fact": "age > 65, medication DrugX, severe reaction"
    }
  ],
  "contexts": [
    "chunk_P000123_note_0",
    "chunk_P000451_note_0",
    "chunk_P000982_note_0"
  ],
  "difficulty": "medium",
  "query_type": "filtered_lookup"
}
```

---

# 9. System Architecture

## 9.1 High-Level Architecture

```text
┌────────────────────┐
│      CLI Driver     │
└─────────┬──────────┘
          │ JobConfig
          ▼
┌────────────────────┐
│ OrchestratorActor   │
│ - creates job       │
│ - spawns workers    │
│ - tracks progress   │
│ - handles shutdown  │
└─────────┬──────────┘
          │
          ├────────────────────────────────────┐
          │                                    │
          ▼                                    ▼
┌────────────────────┐              ┌────────────────────┐
│ Patient Pipeline    │              │ Eval Pipeline       │
└─────────┬──────────┘              └─────────┬──────────┘
          │                                    │
          ▼                                    ▼
┌────────────────────┐              ┌────────────────────┐
│ WriterActor         │◄────────────│ EvalWriterActor      │
└────────────────────┘              └────────────────────┘
```

---

## 9.2 Detailed Actor Pipeline

```text
CLI
 │
 ▼
OrchestratorActor
 │
 ├── PatientProfileActorPool
 │     │
 │     ▼
 │   ConditionAssignmentActorPool
 │     │
 │     ▼
 │   MedicationAssignmentActorPool
 │     │
 │     ▼
 │   ReactionSimulationActorPool
 │     │
 │     ▼
 │   ClinicalNoteActorPool
 │     │
 │     ▼
 │   ChunkingActorPool
 │     │
 │     ▼
 │   PatientWriterActor
 │
 └── EvalOrchestratorActor
       │
       ▼
     EvalQueryActorPool
       │
       ▼
     EvalGroundTruthActorPool
       │
       ▼
     EvalWriterActor
```

---

# 10. Actor Responsibilities

## 10.1 OrchestratorActor

### Purpose

Coordinates the full generation job.

### Responsibilities

- Parse `JobConfig`
- Spawn actor pools
- Partition work into batches
- Track progress
- Handle retries
- Handle backpressure
- Emit job-level metrics
- Initiate graceful shutdown
- Report final summary

### State

```rust
struct OrchestratorState {
    job_id: String,
    config: JobConfig,
    total_patients: usize,
    completed_patients: usize,
    total_evals: usize,
    completed_evals: usize,
    failed_batches: Vec<BatchId>,
    started_at: Instant,
}
```

---

## 10.2 PatientProfileActor

### Purpose

Generate base demographic patient profiles.

### Input

```rust
GeneratePatientBatch {
    batch_id: BatchId,
    start_index: u64,
    size: usize,
    seed: u64,
}
```

### Output

```rust
PatientProfileCreated(PatientProfile)
```

### Fields Generated

- Patient ID
- Age
- Gender
- Region
- Synthetic name, optional
- Baseline risk category

---

## 10.3 ConditionAssignmentActor

### Purpose

Assign comorbidities and medical conditions based on configurable distributions.

### Input

```rust
AssignConditions(PatientProfile)
```

### Output

```rust
ConditionsAssigned(PatientWithConditions)
```

### Example Conditions

- Diabetes
- Hypertension
- Asthma
- Chronic kidney disease
- Coronary artery disease
- COPD
- Depression
- Obesity

---

## 10.4 MedicationAssignmentActor

### Purpose

Assign medications based on conditions and configured probabilities.

### Input

```rust
AssignMedications(PatientWithConditions)
```

### Output

```rust
MedicationsAssigned(PatientWithMedications)
```

### Example Logic

- Diabetes increases probability of Metformin.
- Hypertension increases probability of Lisinopril.
- DrugX can be assigned globally with configurable exposure rate.
- Aspirin can be common among older cardiovascular patients.

---

## 10.5 ReactionSimulationActor

### Purpose

Simulate allergic or adverse reactions using deterministic rules and seeded randomness.

### Input

```rust
SimulateReaction(PatientWithMedications)
```

### Output

```rust
ReactionSimulated(PatientRecordDraft)
```

### Example Rules

```text
If medication contains DrugX:
  base reaction probability = 20%

If age > 65:
  increase severity probability

If diabetes:
  increase reaction probability

If prior allergy risk flag:
  increase reaction probability
```

---

## 10.6 ClinicalNoteActor

### Purpose

Generate natural-language synthetic clinical notes from structured patient records.

### Input

```rust
GenerateClinicalNote(PatientRecordDraft)
```

### Output

```rust
ClinicalNoteGenerated(PatientRecord)
```

### Note Types

- Primary care note
- Allergy note
- Medication history note
- Emergency visit note
- Follow-up note

V1 can generate one note per patient.

Future versions can generate multiple longitudinal notes per patient.

---

## 10.7 ChunkingActor

### Purpose

Create retrieval-ready document chunks for RAG indexing.

### Input

```rust
ChunkPatientRecord(PatientRecord)
```

### Output

```rust
ChunksCreated {
    record: PatientRecord,
    chunks: Vec<CorpusChunk>,
}
```

### Chunk Fields

```rust
struct CorpusChunk {
    chunk_id: String,
    patient_id: String,
    text: String,
    source: String,
    metadata: HashMap<String, String>,
}
```

---

## 10.8 PatientWriterActor

### Purpose

Write patient records and corpus chunks to disk.

### Responsibilities

- Buffered writes
- JSONL output
- CSV output
- Parquet output, optional
- File rotation
- Checkpointing
- Flush on shutdown

### Output Files

```text
patients.jsonl
clinical_notes.jsonl
chunks.jsonl
patients.csv
patients.parquet
```

---

## 10.9 EvalOrchestratorActor

### Purpose

Coordinates eval generation after or during patient generation.

### Modes

| Mode | Description |
|---|---|
| Post-generation | Generate evals after patient dataset is complete |
| Streaming | Generate eval candidates as patient records are produced |
| Hybrid | Stream candidates, finalize after global stats are known |

Recommended V1:

```text
Post-generation eval generation
```

This is simpler and gives access to the full dataset for aggregation queries.

---

## 10.10 EvalQueryActor

### Purpose

Generate query templates and instantiate them against the synthetic corpus.

### Query Types

- Single-patient fact query
- Multi-patient filter query
- Aggregation query
- Multi-hop query
- Negative query
- Evidence query

### Input

```rust
GenerateEvalBatch {
    batch_id: BatchId,
    size: usize,
    seed: u64,
    eval_config: EvalConfig,
}
```

### Output

```rust
EvalQueryGenerated(EvalQueryDraft)
```

---

## 10.11 EvalGroundTruthActor

### Purpose

Resolve ground truth answers for generated eval queries.

### Input

```rust
ResolveGroundTruth(EvalQueryDraft)
```

### Output

```rust
GroundTruthResolved(EvalRecord)
```

---

## 10.12 EvalWriterActor

### Purpose

Write evaluation datasets.

### Output Files

```text
evals.jsonl
ragas_dataset.jsonl
eval_summary.json
```

---

# 11. Message Design

## 11.1 Core Message Envelope

Use an envelope for traceability.

```rust
#[derive(Clone, Debug)]
pub struct Envelope<T> {
    pub job_id: String,
    pub batch_id: u64,
    pub trace_id: String,
    pub payload: T,
}
```

Benefits:

- Easier tracing
- Easier retries
- Easier debugging
- Better logs
- Deterministic batch-level replay

---

## 11.2 Patient Messages

```rust
#[derive(Clone, Debug)]
pub enum PatientMsg {
    GenerateBatch {
        job_id: String,
        batch_id: u64,
        start_index: u64,
        size: usize,
        seed: u64,
    },

    ProfileCreated(Envelope<PatientProfile>),

    ConditionsAssigned(Envelope<PatientWithConditions>),

    MedicationsAssigned(Envelope<PatientWithMedications>),

    ReactionSimulated(Envelope<PatientRecordDraft>),

    ClinicalNoteGenerated(Envelope<PatientRecord>),

    ChunksCreated(Envelope<PatientOutput>),

    BatchCompleted {
        job_id: String,
        batch_id: u64,
        count: usize,
    },

    BatchFailed {
        job_id: String,
        batch_id: u64,
        reason: String,
    },

    Flush,

    Shutdown,
}
```

---

## 11.3 Eval Messages

```rust
#[derive(Clone, Debug)]
pub enum EvalMsg {
    GenerateEvalBatch {
        job_id: String,
        batch_id: u64,
        size: usize,
        seed: u64,
    },

    EvalQueryGenerated(Envelope<EvalQueryDraft>),

    GroundTruthResolved(Envelope<EvalRecord>),

    EvalBatchCompleted {
        job_id: String,
        batch_id: u64,
        count: usize,
    },

    EvalBatchFailed {
        job_id: String,
        batch_id: u64,
        reason: String,
    },

    Flush,

    Shutdown,
}
```

---

# 12. Data Model

## 12.1 PatientProfile

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientProfile {
    pub patient_id: String,
    pub age: u8,
    pub gender: Gender,
    pub region: String,
    pub risk_bucket: RiskBucket,
}
```

---

## 12.2 PatientRecord

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatientRecord {
    pub patient_id: String,
    pub age: u8,
    pub gender: Gender,
    pub region: String,
    pub comorbidities: Vec<String>,
    pub medications: Vec<String>,
    pub allergic_reaction: bool,
    pub reaction_medication: Option<String>,
    pub reaction_type: Option<String>,
    pub reaction_severity: Option<Severity>,
    pub clinical_notes: Vec<ClinicalNote>,
    pub metadata: PatientMetadata,
}
```

---

## 12.3 ClinicalNote

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClinicalNote {
    pub note_id: String,
    pub patient_id: String,
    pub note_type: String,
    pub text: String,
}
```

---

## 12.4 EvalRecord

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalRecord {
    pub eval_id: String,
    pub query: String,
    pub answer: String,
    pub ground_truth_patient_ids: Vec<String>,
    pub ground_truth_facts: Vec<GroundTruthFact>,
    pub contexts: Vec<String>,
    pub context_chunk_ids: Vec<String>,
    pub query_type: EvalQueryType,
    pub difficulty: Difficulty,
    pub metadata: EvalMetadata,
}
```

---

## 12.5 RAGAS-Compatible Output

For RAGAS-style evaluation, produce a flattened JSONL file.

```json
{
  "question": "Which patients over 65 had a severe reaction to DrugX?",
  "answer": "P000123, P000451, P000982",
  "contexts": [
    "Patient P000123 is a 68-year-old female with diabetes...",
    "Patient P000451 is a 72-year-old male with hypertension..."
  ],
  "ground_truth": "P000123, P000451, P000982"
}
```

---

# 13. Configuration

## 13.1 CLI

Primary command:

```bash
Synthetic Patient Data Generator generate \
  --patients 100000 \
  --evals 10000 \
  --seed 42 \
  --output ./data \
  --config ./config/default.toml \
  --format jsonl
```

Additional examples:

```bash
Synthetic Patient Data Generator generate --patients 10000 --seed 7 --output ./demo
```

```bash
Synthetic Patient Data Generator generate \
  --patients 1000000 \
  --evals 100000 \
  --workers 32 \
  --format parquet \
  --output ./large-run
```

```bash
Synthetic Patient Data Generator validate --input ./data/patients.jsonl
```

```bash
Synthetic Patient Data Generator summarize --input ./data/patients.jsonl
```

---

## 13.2 TOML Config

```toml
[job]
batch_size = 1000
patient_count = 100000
eval_count = 10000
seed = 42
output_dir = "./data"
formats = ["jsonl"]

[actors]
profile_workers = 4
condition_workers = 4
medication_workers = 4
reaction_workers = 4
note_workers = 4
chunk_workers = 2
eval_workers = 4
writer_buffer_size = 5000

[demographics]
min_age = 18
max_age = 90
female_probability = 0.52

[conditions]
diabetes = 0.12
hypertension = 0.28
asthma = 0.09
chronic_kidney_disease = 0.04
coronary_artery_disease = 0.06
copd = 0.05
obesity = 0.22

[medications]
drug_x_exposure = 0.05
drug_y_exposure = 0.08
drug_z_exposure = 0.03
aspirin_exposure = 0.18
metformin_exposure_if_diabetes = 0.70

[reactions.drug_x]
reaction_probability = 0.20
severe_probability = 0.15
reaction_types = ["rash", "hives", "shortness of breath"]

[reactions.drug_y]
reaction_probability = 0.08
severe_probability = 0.05
reaction_types = ["nausea", "dizziness", "rash"]

[evals]
easy_probability = 0.40
medium_probability = 0.40
hard_probability = 0.20
include_negative_controls = true
include_aggregation_queries = true
include_multihop_queries = true

[observability]
log_level = "info"
prometheus_enabled = true
prometheus_port = 9090
progress_bar = true
```

---

# 14. Reproducibility Requirements

Reproducibility is a core product feature.

## 14.1 Deterministic Generation

Given the same:

- Seed
- Config
- Version
- Patient count
- Eval count

The system must produce identical output.

## 14.2 Avoid Global RNG

Do not rely on `thread_rng()` for reproducible generation.

Instead, derive deterministic RNG per batch and per patient.

Example approach:

```rust
let batch_seed = root_seed ^ batch_id;
let patient_seed = batch_seed ^ patient_index;
```

Use a seeded RNG such as:

```rust
rand_chacha::ChaCha8Rng
```

Recommended:

```rust
use rand_chacha::ChaCha8Rng;
use rand::{SeedableRng, Rng};

let mut rng = ChaCha8Rng::seed_from_u64(patient_seed);
```

## 14.3 Stable Output Ordering

Concurrency can create nondeterministic write order.

To preserve deterministic output:

Option A: Writer sorts by `patient_id` before final write.

Option B: Writer writes partitioned batch files and merges in batch order.

Recommended V1:

```text
Write batch files → deterministic merge by batch_id
```

Example:

```text
patients_part_000001.jsonl
patients_part_000002.jsonl
patients_part_000003.jsonl
```

Final:

```text
patients.jsonl
```

---

# 15. Backpressure and Batching

## 15.1 Batch Size

Default batch size:

```text
1,000 patients per batch
```

Configurable range:

```text
100 to 50,000
```

## 15.2 Backpressure Strategy

The system should not allow unbounded message growth.

Use:

- Batches instead of one orchestration message per full job
- Writer buffer limits
- Progress acknowledgements
- Bounded channels where needed
- Pause/resume messages if writer falls behind

## 15.3 Writer Backpressure

If writer queue exceeds threshold:

```text
WriterActor → OrchestratorActor: BackpressureHigh
OrchestratorActor pauses new batch dispatch
```

When recovered:

```text
WriterActor → OrchestratorActor: BackpressureNormal
OrchestratorActor resumes dispatch
```

---

# 16. Fault Tolerance

## 16.1 Supervision Philosophy

The actor model should isolate failures to a specific batch or worker.

Expected failure classes:

- Actor panic
- Serialization failure
- File write failure
- Invalid config
- Bad output path
- Worker timeout

## 16.2 Restart Strategy

Actor pools should be supervised by the orchestrator.

If a worker fails:

1. Mark in-flight batch as failed.
2. Restart actor.
3. Retry batch if retry count has not been exceeded.
4. Move to dead-letter log after retry limit.

Config:

```toml
[fault_tolerance]
max_retries_per_batch = 3
retry_backoff_ms = 500
dead_letter_path = "./data/dead_letters.jsonl"
fail_fast = false
```

## 16.3 Checkpointing

For large jobs, checkpoint progress.

Checkpoint file:

```json
{
  "job_id": "job_20260512_001",
  "completed_patient_batches": [0, 1, 2, 3],
  "failed_patient_batches": [],
  "completed_eval_batches": [0, 1],
  "seed": 42,
  "config_hash": "abc123"
}
```

Resume command:

```bash
Synthetic Patient Data Generator resume --checkpoint ./data/checkpoint.json
```

---

# 17. Observability

## 17.1 Structured Logging

Use `tracing`.

Each log should include:

- Job ID
- Batch ID
- Actor name
- Event type
- Count
- Duration
- Error, if applicable

Example:

```json
{
  "level": "INFO",
  "actor": "ReactionSimulationActor",
  "job_id": "job_001",
  "batch_id": 12,
  "event": "batch_completed",
  "records": 1000,
  "duration_ms": 37
}
```

---

## 17.2 Metrics

Expose optional Prometheus metrics.

Metrics:

```text
Synthetic Patient Data Generator_patients_generated_total
Synthetic Patient Data Generator_evals_generated_total
Synthetic Patient Data Generator_batches_completed_total
Synthetic Patient Data Generator_batches_failed_total
Synthetic Patient Data Generator_actor_restarts_total
Synthetic Patient Data Generator_generation_duration_seconds
Synthetic Patient Data Generator_writer_queue_depth
Synthetic Patient Data Generator_records_per_second
Synthetic Patient Data Generator_eval_records_per_second
Synthetic Patient Data Generator_memory_bytes
```

---

## 17.3 Progress Output

CLI should show:

```text
Job: job_001
Patients: 73,000 / 100,000
Evals: 2,500 / 10,000
Throughput: 18,420 records/sec
ETA: 00:00:02
Failed batches: 0
```

---

# 18. Functional Requirements

## FR-001: Generate Patient Records

The system must generate the requested number of synthetic patient records.

Acceptance criteria:

- User can specify patient count.
- Output count equals requested count.
- Each patient has unique `patient_id`.
- Output validates against schema.

---

## FR-002: Generate Clinical Notes

The system must generate at least one synthetic clinical note per patient.

Acceptance criteria:

- Each note references its patient ID.
- Note includes relevant structured facts.
- Reaction notes are consistent with reaction fields.

---

## FR-003: Generate Corpus Chunks

The system must produce retrieval-ready chunks.

Acceptance criteria:

- Each chunk has unique `chunk_id`.
- Each chunk maps to a patient.
- Chunks include metadata.
- Chunk output can be indexed by external RAG systems.

---

## FR-004: Generate Eval Records

The system must generate eval records from generated patient data.

Acceptance criteria:

- Eval count equals requested count.
- Each eval has query, answer, contexts, and ground truth.
- Contexts correspond to generated corpus chunks.
- Ground-truth patient IDs are machine-checkable.

---

## FR-005: Support RAGAS-Compatible Export

The system must export a RAGAS-compatible JSONL file.

Acceptance criteria:

- Each line includes question/query, answer, contexts, and ground truth.
- Output can be loaded by a Python evaluation script.

---

## FR-006: Support Configurable Distributions

The user must be able to control distributions via TOML.

Acceptance criteria:

- Medication exposure rates are configurable.
- Reaction probabilities are configurable.
- Condition probabilities are configurable.
- Eval difficulty mix is configurable.

---

## FR-007: Support Deterministic Seeding

The system must produce identical outputs for the same seed and config.

Acceptance criteria:

- Two runs with same seed produce identical hashes.
- Reproducibility test is part of CI.
- Output ordering is deterministic.

---

## FR-008: Support Multiple Output Formats

V1 must support JSONL.

V1.1 should support CSV.

V1.2 should support Parquet.

Acceptance criteria:

- Format is selectable by CLI.
- Invalid format returns a clear error.
- JSONL is streaming-friendly.

---

## FR-009: Support Job Resume

Large jobs should be resumable.

Acceptance criteria:

- Checkpoint file is written.
- Resume skips completed batches.
- Failed batches can be retried.

---

## FR-010: Provide Summary Report

At job completion, produce a summary.

Example:

```json
{
  "job_id": "job_001",
  "patients_generated": 100000,
  "evals_generated": 10000,
  "drug_x_patients": 5032,
  "drug_x_reactions": 1007,
  "severe_reactions": 148,
  "duration_seconds": 82.4,
  "records_per_second": 1213.5,
  "seed": 42,
  "config_hash": "abc123"
}
```

---

# 19. Non-Functional Requirements

## 19.1 Performance

Initial realistic targets:

| Metric | Target |
|---|---:|
| 100k patients | < 5 minutes on 8-core laptop |
| 10k evals | < 2 minutes |
| JSONL writer throughput | 50k+ records/sec buffered |
| Memory usage for 100k run | < 2 GB |
| Memory usage for 1M run | < 10 GB |
| Actor restart time | < 1 second |

Stretch targets:

| Metric | Target |
|---|---:|
| Patient generation throughput | 20k records/sec on 16-core |
| 1M patients | < 10 minutes |
| 100k evals | < 10 minutes |

---

## 19.2 Reliability

- Worker crash should not kill full job by default.
- Failed batch should retry up to configured limit.
- Writer failure should stop job safely.
- Invalid config should fail before actor startup.
- Partial outputs should be marked clearly.

---

## 19.3 Scalability

The system should scale by increasing actor pool sizes.

Scaling dimensions:

- Profile workers
- Condition workers
- Medication workers
- Reaction workers
- Note workers
- Chunk workers
- Eval workers
- Writer partition count

---

## 19.4 Maintainability

Code should be organized by actor and domain module.

Recommended structure:

```text
src/
  main.rs
  cli.rs
  config.rs
  domain/
    patient.rs
    eval.rs
    clinical_note.rs
  actors/
    orchestrator.rs
    profile.rs
    condition.rs
    medication.rs
    reaction.rs
    note.rs
    chunking.rs
    writer.rs
    eval_orchestrator.rs
    eval_query.rs
    eval_ground_truth.rs
    eval_writer.rs
  output/
    jsonl.rs
    csv.rs
    parquet.rs
  metrics.rs
  rng.rs
  errors.rs
```

---

## 19.5 Security and Safety

Because all data is synthetic:

- No real patient data should be accepted in V1.
- CLI should warn if user attempts to use external input files marked as real.
- Generated names, if used, must be fake.
- README should clearly state that output is synthetic and not clinically validated.

---

# 20. Technical Stack

## 20.1 Core Runtime

| Area | Choice |
|---|---|
| Language | Rust |
| Actor framework | Ractor |
| Async runtime | Tokio |
| Serialization | Serde |
| CLI | clap |
| Config | toml + serde |
| RNG | rand + rand_chacha |
| Logging | tracing |
| Metrics | metrics / prometheus |
| Output | JSONL initially |
| CSV | csv crate |
| Parquet | Polars or parquet crate |

---

## 20.2 Recommended Crates

```toml
[dependencies]
ractor = "0.10"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
rand = "0.8"
rand_chacha = "0.3"
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }
csv = "1"
```

Optional:

```toml
metrics = "0.22"
metrics-exporter-prometheus = "0.13"
polars = "0.40"
```

---

# 21. Improved Ractor Design Notes

## 21.1 Avoid Sending Every Record Through the Orchestrator

The original design routes many messages back through the orchestrator. That can create a bottleneck.

Better:

```text
ProfileActor → ConditionActor → MedicationActor → ReactionActor → NoteActor → WriterActor
```

The orchestrator should coordinate batches and progress, not process every record.

---

## 21.2 Prefer Batch Messages for Throughput

Instead of:

```text
one message per patient
```

Use:

```text
one message per batch of patients
```

Example:

```rust
GeneratePatientBatch { size: 1000 }
```

Then each actor transforms a batch:

```rust
PatientProfilesCreated(Vec<PatientProfile>)
ConditionsAssigned(Vec<PatientWithConditions>)
```

This significantly reduces actor mailbox pressure.

---

## 21.3 Use Deterministic Batch IDs

Patient IDs should be derived from global index.

```rust
patient_id = format!("P{:09}", global_index)
```

Example:

```text
P000000001
P000000002
P000000003
```

This avoids nondeterministic ID creation.

---

## 21.4 Separate Patient Generation and Eval Generation

For V1, generate evals after patient records are complete.

Why:

- Easier ground truth
- Easier aggregation queries
- Easier deterministic behavior
- Easier validation

Later, add streaming eval generation.

---

# 22. Revised Actor Flow

## 22.1 Patient Generation Flow

```text
OrchestratorActor
  │
  │ GeneratePatientBatch(batch_id=0, start=0, size=1000)
  ▼
PatientProfileActor
  │
  │ PatientProfileBatchCreated
  ▼
ConditionAssignmentActor
  │
  │ ConditionBatchAssigned
  ▼
MedicationAssignmentActor
  │
  │ MedicationBatchAssigned
  ▼
ReactionSimulationActor
  │
  │ ReactionBatchSimulated
  ▼
ClinicalNoteActor
  │
  │ ClinicalNoteBatchGenerated
  ▼
ChunkingActor
  │
  │ ChunkBatchCreated
  ▼
PatientWriterActor
  │
  │ BatchWritten
  ▼
OrchestratorActor
```

---

## 22.2 Eval Generation Flow

```text
EvalOrchestratorActor
  │
  │ GenerateEvalBatch
  ▼
EvalQueryActor
  │
  │ EvalQueryBatchGenerated
  ▼
EvalGroundTruthActor
  │
  │ EvalGroundTruthBatchResolved
  ▼
EvalWriterActor
  │
  │ EvalBatchWritten
  ▼
EvalOrchestratorActor
```

---

# 23. Better Rust Message Skeleton

```rust
#[derive(Clone, Debug)]
pub enum PipelineMsg {
    GeneratePatientBatch {
        job_id: String,
        batch_id: u64,
        start_index: u64,
        size: usize,
        seed: u64,
    },

    ProfileBatchCreated {
        job_id: String,
        batch_id: u64,
        profiles: Vec<PatientProfile>,
    },

    ConditionBatchAssigned {
        job_id: String,
        batch_id: u64,
        patients: Vec<PatientWithConditions>,
    },

    MedicationBatchAssigned {
        job_id: String,
        batch_id: u64,
        patients: Vec<PatientWithMedications>,
    },

    ReactionBatchSimulated {
        job_id: String,
        batch_id: u64,
        patients: Vec<PatientRecordDraft>,
    },

    ClinicalNoteBatchGenerated {
        job_id: String,
        batch_id: u64,
        records: Vec<PatientRecord>,
    },

    ChunkBatchCreated {
        job_id: String,
        batch_id: u64,
        output: Vec<PatientOutput>,
    },

    BatchWritten {
        job_id: String,
        batch_id: u64,
        records_written: usize,
    },

    BatchFailed {
        job_id: String,
        batch_id: u64,
        actor: String,
        reason: String,
    },

    Shutdown,
}
```

---

# 24. MVP Product Definition

## MVP Goal

Build a CLI-based Rust system that can generate:

```text
100,000 synthetic patients
10,000 eval records
JSONL corpus chunks
RAGAS-compatible eval file
Deterministic output from seed
Basic actor supervision and metrics
```

---

## MVP Features

### Must Have

- CLI command
- TOML config
- Ractor actor pipeline
- Batch-based patient generation
- Medication and reaction simulation
- Clinical note generation
- JSONL output
- Eval query generation
- Ground truth generation
- RAGAS-compatible export
- Seed reproducibility
- Summary report
- Structured logs
- Basic metrics

### Should Have

- CSV export
- Checkpoint/resume
- Prometheus endpoint
- Fault injection test
- Progress bar
- Batch retry

### Could Have

- Parquet export
- Multi-file partitioning
- Interactive dashboard
- LLM-assisted query generation
- Docker image
- Python loader notebook

### Won’t Have in MVP

- Real patient data ingestion
- FHIR support
- HIPAA compliance workflows
- Clinical accuracy guarantees
- Distributed multi-node generation
- Web UI

---

# 25. Acceptance Criteria

## 25.1 Generation

```text
Given I run the generator with 100,000 patients
When the job completes
Then patients.jsonl contains exactly 100,000 records
And each patient_id is unique
And each patient has at least one clinical note
And chunks.jsonl contains at least 100,000 chunks
```

---

## 25.2 Eval Generation

```text
Given a generated patient dataset
When I request 10,000 eval records
Then evals.jsonl contains exactly 10,000 records
And every eval has a query, answer, contexts, and ground_truth
And every referenced context exists in chunks.jsonl
```

---

## 25.3 Reproducibility

```text
Given the same config and seed
When I run generation twice
Then the SHA256 hash of patients.jsonl is identical
And the SHA256 hash of evals.jsonl is identical
```

---

## 25.4 Fault Tolerance

```text
Given fault injection is enabled
When 20% of worker batches fail once
Then the orchestrator retries failed batches
And the final output still contains the requested number of records
And failed retries are visible in the summary report
```

---

## 25.5 Performance

```text
Given an 8-core development machine
When I generate 100,000 patients and 10,000 evals
Then the job completes in under 7 minutes
```

---

# 26. Implementation Plan

## Phase 1: Foundation

### Deliverables

- CLI scaffold
- Config loader
- Domain structs
- JSONL writer
- Seeded RNG helper
- Basic Ractor actor startup

### Exit Criteria

```text
cargo run -- generate --patients 1000 --seed 42 --output ./data
```

Produces valid `patients.jsonl`.

---

## Phase 2: Patient Pipeline

### Deliverables

- Profile actor
- Condition actor
- Medication actor
- Reaction actor
- Clinical note actor
- Chunking actor
- Writer actor
- Batch-based pipeline

### Exit Criteria

Generate 100k patients with clinical notes and chunks.

---

## Phase 3: Eval Pipeline

### Deliverables

- Eval orchestrator
- Eval query actor
- Ground truth actor
- Eval writer
- RAGAS export

### Exit Criteria

Generate 10k eval records from patient data.

---

## Phase 4: Reliability and Observability

### Deliverables

- Structured logs
- Progress reporting
- Summary report
- Retry logic
- Checkpoint file
- Reproducibility test
- Fault injection test

### Exit Criteria

System survives worker failure and produces deterministic output.

---

## Phase 5: Polish for Blog / Demo

### Deliverables

- Architecture diagrams
- README
- Example configs
- Benchmark results
- Sample generated data
- Python RAGAS loader notebook
- Blog-ready explanation

---

# 27. Example Output Files

```text
data/
  config.resolved.toml
  summary.json
  patients.jsonl
  clinical_notes.jsonl
  chunks.jsonl
  evals.jsonl
  ragas_dataset.jsonl
  checkpoint.json
  logs/
    job_001.log
```

---

# 28. Example Summary Report

```json
{
  "job_id": "job_001",
  "seed": 42,
  "config_hash": "ac8912ef",
  "patients_requested": 100000,
  "patients_generated": 100000,
  "evals_requested": 10000,
  "evals_generated": 10000,
  "chunks_generated": 100000,
  "drug_x_patients": 4978,
  "drug_x_reactions": 996,
  "drug_x_severe_reactions": 151,
  "duration_seconds": 284.3,
  "records_per_second": 351.7,
  "failed_batches": 0,
  "retried_batches": 0,
  "output_files": [
    "patients.jsonl",
    "chunks.jsonl",
    "evals.jsonl",
    "ragas_dataset.jsonl"
  ]
}
```

---

# 29. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---:|---|
| Actor pipeline becomes too complex | High | Use batch messages and clear actor boundaries |
| Output order becomes nondeterministic | High | Partition by batch and merge deterministically |
| Writer becomes bottleneck | Medium | Buffered writer, partitioned output files |
| Reproducibility breaks due to RNG | High | Use seeded per-batch/per-record RNG |
| Eval generation requires global stats | Medium | Generate evals after patient dataset is complete |
| Parquet implementation slows MVP | Medium | Ship JSONL first |
| Medical realism challenged | Medium | Clearly label as synthetic benchmark data |
| Too many actor types for V1 | Medium | Keep MVP actors minimal and add refinements later |

---

# 30. Tightened Success Criteria

The project is successful when:

```text
1. A user can generate 100k patients and 10k evals from one CLI command.
2. The output is deterministic for a fixed seed.
3. The architecture clearly demonstrates Ractor actor supervision and concurrency.
4. The generated evals are usable for RAG/RAGAS-style benchmarking.
5. The system produces JSONL files that can be indexed by a retrieval system.
6. The implementation is understandable enough for a technical blog.
7. Worker failure can be simulated and recovered without corrupting output.
```

---

# 31. Recommended First Build

Do **not** start with all actors.

Start with a narrow but complete vertical slice:

```text
OrchestratorActor
  → PatientProfileActor
  → MedicationReactionActor
  → ClinicalNoteActor
  → WriterActor
```

Then add:

```text
EvalOrchestratorActor
  → EvalQueryActor
  → EvalGroundTruthActor
  → EvalWriterActor
```

This gives you a working end-to-end system quickly while still preserving the actor-system story.

---

# 32. Final Recommended Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                         CLI Driver                            │
│  Synthetic Patient Data Generator generate --patients 100k --evals 10k --seed 42    │
└──────────────────────────────┬───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────────────────────────────────────┐
│                     OrchestratorActor                         │
│  - validates config                                            │
│  - spawns actor pools                                          │
│  - dispatches batches                                          │
│  - tracks progress                                             │
│  - retries failed batches                                      │
│  - writes summary                                              │
└──────────────────────────────┬───────────────────────────────┘
                               │
        ┌──────────────────────┴──────────────────────┐
        ▼                                             ▼
┌──────────────────────┐                    ┌──────────────────────┐
│ Patient Data Pipeline │                    │ EvalSet Pipeline      │
└──────────┬───────────┘                    └──────────┬───────────┘
           │                                           │
           ▼                                           ▼
┌──────────────────────┐                    ┌──────────────────────┐
│ ProfileActorPool      │                    │ EvalQueryActorPool    │
└──────────┬───────────┘                    └──────────┬───────────┘
           │                                           │
           ▼                                           ▼
┌──────────────────────┐                    ┌──────────────────────┐
│ ConditionActorPool    │                    │ GroundTruthActorPool  │
└──────────┬───────────┘                    └──────────┬───────────┘
           │                                           │
           ▼                                           ▼
┌──────────────────────┐                    ┌──────────────────────┐
│ MedicationActorPool   │                    │ EvalWriterActor       │
└──────────┬───────────┘                    └──────────────────────┘
           │
           ▼
┌──────────────────────┐
│ ReactionActorPool     │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ ClinicalNoteActorPool │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ ChunkingActorPool     │
└──────────┬───────────┘
           │
           ▼
┌──────────────────────┐
│ PatientWriterActor    │
└──────────────────────┘
```

---

# 33. One-Line Product Definition

> **Synthetic Patient Data Generator is a Rust + Ractor synthetic healthcare data factory that generates reproducible patient records, clinical notes, retrieval chunks, and RAG evaluation datasets at scale using a supervised actor pipeline.**
