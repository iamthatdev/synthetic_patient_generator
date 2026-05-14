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

### From Binary Release

Download the latest release from [Releases](https://github.com/your-org/synthetic_patient_generator/releases).

## Usage

```bash
# Generate 1000 patients
synthetic-patient-gen --patients 1000 --output ./data

# With custom configuration
synthetic-patient-gen --patients 5000 --config ./config/custom.toml --output ./data

# Generate with evaluation set
synthetic-patient-gen --patients 1000 --with-evals --output ./data
```

## Output Files

| File | Description |
|------|-------------|
| `patients.jsonl` | Patient records with demographics, conditions, medications |
| `chunks.jsonl` | Clinical note chunks with metadata |
| `evals.jsonl` | Evaluation questions and ground truth |
| `guardrail_report.json` | Guardrail violation summary |

## Configuration

See `config/default.toml` for all configuration options.

## Development

```bash
# Run tests
cargo test

# Run with debug output
cargo run -- --patients 10 --output ./test-data

# Build release binary
cargo build --release
```

## License

MIT

## Related Projects

- [healthcare-rag-showcase](https://github.com/your-org/healthcare-rag-showcase) - RAG application using this generator
