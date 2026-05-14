use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobConfig {
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_patient_count")]
    pub patient_count: u64,
    #[serde(default = "default_eval_count")]
    pub eval_count: u64,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    #[serde(default = "default_formats")]
    pub formats: Vec<String>,

    #[serde(default)]
    pub actors: ActorConfig,
    #[serde(default)]
    pub demographics: DemographicsConfig,
    #[serde(default)]
    pub conditions: ConditionsConfig,
    #[serde(default)]
    pub medications: MedicationsConfig,
    #[serde(default)]
    pub reactions: ReactionsConfig,
    #[serde(default)]
    pub evals: EvalsConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub fault_tolerance: FaultToleranceConfig,
    #[serde(default)]
    pub guardrails: GuardrailsConfig,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            patient_count: 10000,
            eval_count: 1000,
            seed: 42,
            output_dir: PathBuf::from("./data"),
            formats: vec!["jsonl".into()],
            actors: ActorConfig::default(),
            demographics: DemographicsConfig::default(),
            conditions: ConditionsConfig::default(),
            medications: MedicationsConfig::default(),
            reactions: ReactionsConfig::default(),
            evals: EvalsConfig::default(),
            observability: ObservabilityConfig::default(),
            fault_tolerance: FaultToleranceConfig::default(),
            guardrails: GuardrailsConfig::default(),
        }
    }
}

fn default_batch_size() -> usize { 1000 }
fn default_patient_count() -> u64 { 10000 }
fn default_eval_count() -> u64 { 1000 }
fn default_seed() -> u64 { 42 }
fn default_output_dir() -> PathBuf { PathBuf::from("./data") }
fn default_formats() -> Vec<String> { vec!["jsonl".into()] }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorConfig {
    #[serde(default = "default_workers")]
    pub profile_workers: usize,
    #[serde(default = "default_workers")]
    pub condition_workers: usize,
    #[serde(default = "default_workers")]
    pub medication_workers: usize,
    #[serde(default = "default_workers")]
    pub reaction_workers: usize,
    #[serde(default = "default_workers")]
    pub note_workers: usize,
    #[serde(default = "default_two")]
    pub chunk_workers: usize,
    #[serde(default = "default_workers")]
    pub eval_workers: usize,
    #[serde(default = "default_writer_buffer")]
    pub writer_buffer_size: usize,
}

impl Default for ActorConfig {
    fn default() -> Self {
        Self {
            profile_workers: 4,
            condition_workers: 4,
            medication_workers: 4,
            reaction_workers: 4,
            note_workers: 4,
            chunk_workers: 2,
            eval_workers: 4,
            writer_buffer_size: 5000,
        }
    }
}

fn default_workers() -> usize { 4 }
fn default_two() -> usize { 2 }
fn default_writer_buffer() -> usize { 5000 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemographicsConfig {
    #[serde(default = "default_min_age")]
    pub min_age: u8,
    #[serde(default = "default_max_age")]
    pub max_age: u8,
    #[serde(default = "default_female_prob")]
    pub female_probability: f64,
}

impl Default for DemographicsConfig {
    fn default() -> Self {
        Self {
            min_age: 18,
            max_age: 90,
            female_probability: 0.52,
        }
    }
}

fn default_min_age() -> u8 { 18 }
fn default_max_age() -> u8 { 90 }
fn default_female_prob() -> f64 { 0.52 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionsConfig {
    #[serde(default = "default_diabetes")]
    pub diabetes: f64,
    #[serde(default = "default_hypertension")]
    pub hypertension: f64,
    #[serde(default = "default_asthma")]
    pub asthma: f64,
    #[serde(default = "default_ckd")]
    pub chronic_kidney_disease: f64,
    #[serde(default = "default_cad")]
    pub coronary_artery_disease: f64,
    #[serde(default = "default_copd")]
    pub copd: f64,
    #[serde(default = "default_obesity")]
    pub obesity: f64,
}

impl Default for ConditionsConfig {
    fn default() -> Self {
        Self {
            diabetes: 0.12,
            hypertension: 0.28,
            asthma: 0.09,
            chronic_kidney_disease: 0.04,
            coronary_artery_disease: 0.06,
            copd: 0.05,
            obesity: 0.22,
        }
    }
}

fn default_diabetes() -> f64 { 0.12 }
fn default_hypertension() -> f64 { 0.28 }
fn default_asthma() -> f64 { 0.09 }
fn default_ckd() -> f64 { 0.04 }
fn default_cad() -> f64 { 0.06 }
fn default_copd() -> f64 { 0.05 }
fn default_obesity() -> f64 { 0.22 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedicationsConfig {
    #[serde(default = "default_drug_x")]
    pub drug_x_exposure: f64,
    #[serde(default = "default_drug_y")]
    pub drug_y_exposure: f64,
    #[serde(default = "default_drug_z")]
    pub drug_z_exposure: f64,
    #[serde(default = "default_aspirin")]
    pub aspirin_exposure: f64,
    #[serde(default = "default_metformin")]
    pub metformin_exposure_if_diabetes: f64,
}

impl Default for MedicationsConfig {
    fn default() -> Self {
        Self {
            drug_x_exposure: 0.05,
            drug_y_exposure: 0.08,
            drug_z_exposure: 0.03,
            aspirin_exposure: 0.18,
            metformin_exposure_if_diabetes: 0.70,
        }
    }
}

fn default_drug_x() -> f64 { 0.05 }
fn default_drug_y() -> f64 { 0.08 }
fn default_drug_z() -> f64 { 0.03 }
fn default_aspirin() -> f64 { 0.18 }
fn default_metformin() -> f64 { 0.70 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReactionDrugConfig {
    #[serde(default = "default_reaction_prob")]
    pub reaction_probability: f64,
    #[serde(default = "default_severe_prob")]
    pub severe_probability: f64,
    #[serde(default = "default_reaction_types")]
    pub reaction_types: Vec<String>,
}

fn default_reaction_prob() -> f64 { 0.20 }
fn default_severe_prob() -> f64 { 0.15 }
fn default_reaction_types() -> Vec<String> {
    vec!["rash".into(), "hives".into(), "shortness of breath".into()]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReactionsConfig {
    #[serde(default = "default_drug_x_reaction")]
    pub drug_x: ReactionDrugConfig,
    #[serde(default = "default_drug_y_reaction")]
    pub drug_y: ReactionDrugConfig,
}

impl Default for ReactionsConfig {
    fn default() -> Self {
        Self {
            drug_x: ReactionDrugConfig {
                reaction_probability: 0.20,
                severe_probability: 0.15,
                reaction_types: vec![
                    "rash".into(),
                    "hives".into(),
                    "shortness of breath".into(),
                ],
            },
            drug_y: ReactionDrugConfig {
                reaction_probability: 0.08,
                severe_probability: 0.05,
                reaction_types: vec!["nausea".into(), "dizziness".into(), "rash".into()],
            },
        }
    }
}

fn default_drug_x_reaction() -> ReactionDrugConfig {
    ReactionDrugConfig {
        reaction_probability: 0.20,
        severe_probability: 0.15,
        reaction_types: vec!["rash".into(), "hives".into(), "shortness of breath".into()],
    }
}

fn default_drug_y_reaction() -> ReactionDrugConfig {
    ReactionDrugConfig {
        reaction_probability: 0.08,
        severe_probability: 0.05,
        reaction_types: vec!["nausea".into(), "dizziness".into(), "rash".into()],
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvalsConfig {
    #[serde(default = "default_easy_prob")]
    pub easy_probability: f64,
    #[serde(default = "default_medium_prob")]
    pub medium_probability: f64,
    #[serde(default = "default_hard_prob")]
    pub hard_probability: f64,
    #[serde(default = "default_true")]
    pub include_negative_controls: bool,
    #[serde(default = "default_true")]
    pub include_aggregation_queries: bool,
    #[serde(default = "default_true")]
    pub include_multihop_queries: bool,
}

impl Default for EvalsConfig {
    fn default() -> Self {
        Self {
            easy_probability: 0.40,
            medium_probability: 0.40,
            hard_probability: 0.20,
            include_negative_controls: true,
            include_aggregation_queries: true,
            include_multihop_queries: true,
        }
    }
}

fn default_easy_prob() -> f64 { 0.40 }
fn default_medium_prob() -> f64 { 0.40 }
fn default_hard_prob() -> f64 { 0.20 }
fn default_true() -> bool { true }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub prometheus_enabled: bool,
    #[serde(default = "default_prometheus_port")]
    pub prometheus_port: u16,
    #[serde(default = "default_true")]
    pub progress_bar: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".into(),
            prometheus_enabled: false,
            prometheus_port: 9090,
            progress_bar: true,
        }
    }
}

fn default_log_level() -> String { "info".into() }
fn default_prometheus_port() -> u16 { 9090 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FaultToleranceConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries_per_batch: u32,
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,
    #[serde(default = "default_dead_letter_path")]
    pub dead_letter_path: PathBuf,
    #[serde(default)]
    pub fail_fast: bool,
}

impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            max_retries_per_batch: 3,
            retry_backoff_ms: 500,
            dead_letter_path: PathBuf::from("./data/dead_letters.jsonl"),
            fail_fast: false,
        }
    }
}

fn default_max_retries() -> u32 { 3 }
fn default_retry_backoff_ms() -> u64 { 500 }
fn default_dead_letter_path() -> PathBuf { PathBuf::from("./data/dead_letters.jsonl") }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub fail_on_error: bool,
    #[serde(default = "default_true")]
    pub generate_report: bool,

    #[serde(default)]
    pub pii: PiiConfig,
    #[serde(default)]
    pub content_policy: ContentPolicyConfig,
    #[serde(default)]
    pub plausibility: PlausibilityConfig,
    #[serde(default)]
    pub distribution: DistributionConfig,
    #[serde(default)]
    pub uniqueness: UniquenessConfig,
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fail_on_error: false,
            generate_report: true,
            pii: PiiConfig::default(),
            content_policy: ContentPolicyConfig::default(),
            plausibility: PlausibilityConfig::default(),
            distribution: DistributionConfig::default(),
            uniqueness: UniquenessConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PiiConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub detect_ssn: bool,
    #[serde(default = "default_true")]
    pub detect_phone: bool,
    #[serde(default = "default_true")]
    pub detect_email: bool,
}

impl Default for PiiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_ssn: true,
            detect_phone: true,
            detect_email: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentPolicyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_content_self_harm")]
    pub self_harm_terms: Vec<String>,
    #[serde(default = "default_content_violence")]
    pub violence_terms: Vec<String>,
}

impl Default for ContentPolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            self_harm_terms: vec!["suicide".into(), "self-harm".into()],
            violence_terms: vec!["homicide".into(), "assault".into()],
        }
    }
}

fn default_content_self_harm() -> Vec<String> { vec!["suicide".into(), "self-harm".into()] }
fn default_content_violence() -> Vec<String> { vec!["homicide".into(), "assault".into()] }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlausibilityConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_comorbidities_young")]
    pub max_comorbidities_young: u8,
    #[serde(default = "default_max_comorbidities_elderly")]
    pub max_comorbidities_elderly: u8,
    #[serde(default = "default_true")]
    pub check_gender_conditions: bool,
}

impl Default for PlausibilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_comorbidities_young: 4,
            max_comorbidities_elderly: 10,
            check_gender_conditions: true,
        }
    }
}

fn default_max_comorbidities_young() -> u8 { 4 }
fn default_max_comorbidities_elderly() -> u8 { 10 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributionConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            tolerance: 0.05,
        }
    }
}

fn default_tolerance() -> f64 { 0.05 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UniquenessConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for UniquenessConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub fn load_config(path: Option<&std::path::Path>) -> Result<JobConfig, crate::errors::AppError> {
    match path {
        Some(p) => {
            let content = std::fs::read_to_string(p)
                .map_err(|e| crate::errors::AppError::Config(format!("Failed to read config file {}: {}", p.display(), e)))?;
            toml::from_str(&content)
                .map_err(|e| crate::errors::AppError::Config(format!("Failed to parse config: {}", e)))
        }
        None => Ok(JobConfig::default()),
    }
}

pub fn merge_cli_overrides(config: &mut JobConfig, patients: Option<u64>, evals: Option<u64>, seed: Option<u64>, output: Option<PathBuf>) {
    if let Some(p) = patients {
        config.patient_count = p;
    }
    if let Some(e) = evals {
        config.eval_count = e;
    }
    if let Some(s) = seed {
        config.seed = s;
    }
    if let Some(o) = output {
        config.output_dir = o;
    }
}
