mod actors;
mod cli;
mod config;
mod data;
mod domain;
mod errors;
mod eval_generation;
mod generation;
mod guardrails;
mod output;
mod rng;

use clap::Parser;
use config::JobConfig;
use ractor::Actor;

fn main() -> Result<(), errors::AppError> {
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Generate {
            patients,
            evals,
            seed,
            output,
            config,
            format: _fmt,
        } => run_generate(patients, evals, seed, output, config),
        cli::Commands::Resume { checkpoint: _ } => {
            eprintln!("Resume not yet implemented");
            std::process::exit(1);
        }
        cli::Commands::Validate { input: _ } => {
            eprintln!("Validate not yet implemented");
            std::process::exit(1);
        }
        cli::Commands::Summarize { input: _ } => {
            eprintln!("Summarize not yet implemented");
            std::process::exit(1);
        }
    }
}

fn run_generate(
    patients: Option<u64>,
    evals: Option<u64>,
    seed: Option<u64>,
    output: Option<std::path::PathBuf>,
    config_path: Option<std::path::PathBuf>,
) -> Result<(), errors::AppError> {
    let mut config = config::load_config(config_path.as_deref())?;
    config::merge_cli_overrides(&mut config, patients, evals, seed, output);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(generate(config))
}

async fn generate(config: JobConfig) -> Result<(), errors::AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new(&config.observability.log_level)
            }),
        )
        .init();

    tracing::info!(
        patient_count = config.patient_count,
        eval_count = config.eval_count,
        seed = config.seed,
        output_dir = %config.output_dir.display(),
        "Starting generation (actor pipeline)"
    );

    let (_orchestrator, handle) = actors::orchestrator::OrchestratorActor::spawn(
        Some("orchestrator".to_string()),
        actors::orchestrator::OrchestratorActor,
        config,
    )
    .await
    .map_err(|e| errors::AppError::ActorSpawn(format!("Failed to spawn orchestrator: {}", e)))?;

    handle
        .await
        .map_err(|e| errors::AppError::ActorSpawn(format!("Orchestrator panicked: {}", e)))?;

    Ok(())
}
