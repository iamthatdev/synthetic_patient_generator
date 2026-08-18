# Synthetic Patient Generator

A high-performance Rust tool for generating synthetic healthcare patient data with built-in guardrails and validation.

## Features

- **Census-based Name Generation**: Realistic patient names from demographic data
- **Clinical Note Generation**: Condition-specific clinical narratives
- **Guardrail Checking**: PII detection, content policy, plausibility, distribution, uniqueness
- **JSONL Output**: Structured data ready for RAG applications
- **Batch Processing**: Generate thousands of patients efficiently

## Installation

### From Source

```bash
cargo install --path .
```

This installs the `synthetic-patient-gen` binary into `~/.cargo/bin`. Without installing,
run it through Cargo instead: `cargo run --release -- generate ...`.

### From Binary Release

Download the latest release from [Releases](https://github.com/your-org/synthetic_patient_generator/releases).

## Usage

```bash
# Generate 1000 patients (plus the default 1000 evals)
synthetic-patient-gen generate --patients 1000 --output ./data

# With custom configuration
synthetic-patient-gen generate --patients 5000 --config ./config/custom.toml --output ./data

# Control the size of the evaluation set (0 disables it)
synthetic-patient-gen generate --patients 1000 --evals 5000 --output ./data
```

Other subcommands: `validate --input`, `summarize --input`, `resume --checkpoint`.
Run `synthetic-patient-gen --help` for the full list.

## Output Files

| File | Description |
|------|-------------|
| `patients.jsonl` | Patient records with demographics, conditions, medications |
| `clinical_notes.jsonl` | Full clinical notes per patient |
| `chunks.jsonl` | Clinical note chunks with metadata |
| `evals.jsonl` | Evaluation questions and ground truth |
| `ragas_dataset.jsonl` | Eval set in RAGAS format |
| `guardrail_report.json` | Guardrail violation summary |
| `summary.json` | Job metadata: counts, seed, duration, output files |

## Configuration

`config/default.toml` documents every option, but it is **not** loaded automatically -
pass it explicitly with `--config ./config/default.toml`. With no `--config`, the built-in
defaults apply (10,000 patients, 1,000 evals, seed 42, output `./data`); CLI flags
override whichever source is used.

## Development

```bash
# Run tests
cargo test

# Run with debug output
cargo run -- generate --patients 10 --output ./test-data

# Build release binary
cargo build --release
```

## License

MIT

## Related Projects

- [healthcare-rag-showcase](https://github.com/your-org/healthcare-rag-showcase) - RAG application using this generator
