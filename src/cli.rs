use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "synthetic-patient-gen")]
#[command(about = "Ractor-powered synthetic healthcare data & eval set generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate synthetic patient data and eval sets
    Generate {
        /// Number of patients to generate
        #[arg(long, short)]
        patients: Option<u64>,

        /// Number of eval records to generate
        #[arg(long, short)]
        evals: Option<u64>,

        /// Random seed for reproducibility
        #[arg(long, short)]
        seed: Option<u64>,

        /// Output directory
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Path to TOML config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Output format (jsonl, csv)
        #[arg(long, default_value = "jsonl")]
        format: String,
    },
    /// Resume a previously interrupted job
    Resume {
        /// Path to checkpoint file
        #[arg(long)]
        checkpoint: PathBuf,
    },
    /// Validate generated output files
    Validate {
        /// Path to patients.jsonl
        #[arg(long)]
        input: PathBuf,
    },
    /// Print summary of a generated dataset
    Summarize {
        /// Path to summary.json or output directory
        #[arg(long)]
        input: PathBuf,
    },
}
