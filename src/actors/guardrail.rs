//! GuardrailActor - validates patient records before they're chunked and indexed.

use ractor::{Actor, ActorProcessingErr, ActorRef};
use crate::actors::messages::PipelineMsg;
use crate::domain::guardrail::{
    DistributionDeviation, GuardrailReport, PolicyViolationExample, ViolationBatch,
    ViolationSeverity, ViolationType,
};
use crate::guardrails::{pii, content, plausibility, distribution, uniqueness};
use crate::output::guardrail_report::write_guardrail_report;
use std::path::PathBuf;

/// Cap on stored examples per check so the report stays readable at large patient counts.
const MAX_EXAMPLES: usize = 100;

/// Configuration for guardrail checks
#[derive(Clone, Debug)]
pub struct GuardrailConfig {
    pub pii_check_enabled: bool,
    pub content_policy_enabled: bool,
    pub plausibility_check_enabled: bool,
    pub distribution_check_enabled: bool,
    pub uniqueness_check_enabled: bool,
    pub distribution_tolerance: f64,
    pub fail_on_error: bool,
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            pii_check_enabled: true,
            content_policy_enabled: true,
            plausibility_check_enabled: true,
            distribution_check_enabled: true,
            uniqueness_check_enabled: true,
            distribution_tolerance: 0.05,
            fail_on_error: false,
        }
    }
}

pub struct GuardrailActor;

pub struct GuardrailActorArgs {
    pub downstream: ActorRef<PipelineMsg>,
    pub config: GuardrailConfig,
    pub conditions_config: crate::config::ConditionsConfig,
    pub job_id: String,
    pub output_dir: PathBuf,
}

pub struct GuardrailActorState {
    downstream: ActorRef<PipelineMsg>,
    config: GuardrailConfig,
    conditions_config: distribution::ConditionsConfig,
    output_dir: PathBuf,
    report: GuardrailReport,
    total_check_time_ms: u64,
}

impl GuardrailActorState {
    /// Folds one batch's violations into the running report.
    fn record_batch(&mut self, batch: &ViolationBatch, records_checked: usize, elapsed_ms: u64) {
        let summary = &mut self.report.summary;
        summary.total_checked += records_checked;
        summary.rejected += batch.rejected_ids.len();
        summary.flagged += batch.warnings.len() + batch.errors.len();
        summary.passed = summary.total_checked - summary.rejected;

        self.total_check_time_ms += elapsed_ms;
        self.report.performance_metrics.total_check_time_ms = self.total_check_time_ms;
        self.report.performance_metrics.avg_time_per_record_ms = if summary.total_checked > 0 {
            self.total_check_time_ms as f64 / summary.total_checked as f64
        } else {
            0.0
        };

        let checks = &mut self.report.checks;
        for violation in batch.warnings.iter().chain(batch.errors.iter()) {
            match &violation.violation_type {
                ViolationType::PiiScan { pattern_type, .. } => {
                    checks.pii_scan.triggered += 1;
                    if checks.pii_scan.violations.len() < MAX_EXAMPLES {
                        checks
                            .pii_scan
                            .violations
                            .push(format!("{}: {}", violation.patient_id, pattern_type));
                    }
                }
                ViolationType::NameSafety { blocked_name } => {
                    checks.name_safety.triggered += 1;
                    if checks.name_safety.blocked_names_found.len() < MAX_EXAMPLES {
                        checks
                            .name_safety
                            .blocked_names_found
                            .push(blocked_name.clone());
                    }
                }
                ViolationType::ContentPolicy {
                    category,
                    term,
                    context,
                } => {
                    checks.content_policy.triggered += 1;
                    *checks
                        .content_policy
                        .violations_by_category
                        .entry(category.clone())
                        .or_insert(0) += 1;
                    if checks.content_policy.examples.len() < MAX_EXAMPLES {
                        checks.content_policy.examples.push(PolicyViolationExample {
                            patient_id: violation.patient_id.clone(),
                            category: category.clone(),
                            term: term.clone(),
                            context: context.clone(),
                        });
                    }
                }
                ViolationType::Plausibility { rule, .. } => {
                    checks.plausibility.triggered += 1;
                    *checks
                        .plausibility
                        .violations_by_type
                        .entry(rule.clone())
                        .or_insert(0) += 1;
                }
                ViolationType::Distribution {
                    metric,
                    expected,
                    actual,
                    deviation,
                } => {
                    checks.distribution.triggered += 1;
                    if checks.distribution.deviations.len() < MAX_EXAMPLES {
                        checks.distribution.deviations.push(DistributionDeviation {
                            metric: metric.clone(),
                            expected: *expected,
                            actual: *actual,
                            deviation: *deviation,
                            within_tolerance: *deviation <= self.config.distribution_tolerance,
                        });
                    }
                }
                ViolationType::Uniqueness { duplicate_id } => {
                    checks.uniqueness.triggered += 1;
                    if checks.uniqueness.duplicate_ids.len() < MAX_EXAMPLES {
                        checks.uniqueness.duplicate_ids.push(duplicate_id.clone());
                    }
                }
            }
        }

        checks.pii_scan.enabled = self.config.pii_check_enabled;
        checks.name_safety.enabled = self.config.pii_check_enabled;
        checks.content_policy.enabled = self.config.content_policy_enabled;
        checks.plausibility.enabled = self.config.plausibility_check_enabled;
        checks.distribution.enabled = self.config.distribution_check_enabled;
        checks.uniqueness.enabled = self.config.uniqueness_check_enabled;
    }
}

#[async_trait::async_trait]
impl Actor for GuardrailActor {
    type Msg = PipelineMsg;
    type State = GuardrailActorState;
    type Arguments = GuardrailActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(GuardrailActorState {
            downstream: args.downstream,
            config: args.config,
            conditions_config: distribution::ConditionsConfig::from(&args.conditions_config),
            output_dir: args.output_dir,
            report: GuardrailReport::new(args.job_id),
            total_check_time_ms: 0,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            PipelineMsg::ClinicalNoteBatchGenerated {
                job_id,
                batch_id,
                records,
            } => {
                let started = std::time::Instant::now();
                let records_checked = records.len();
                let mut passed_records = Vec::new();
                let mut batch_violations = ViolationBatch::new();

                for record in records {
                    let mut record_violations = Vec::new();

                    if state.config.pii_check_enabled {
                        for note in &record.clinical_notes {
                            let pii_result = pii::scan_and_redact_pii(
                                &note.text,
                                record.patient_id.clone(),
                            );
                            record_violations.extend(pii_result.violations);
                        }
                    }

                    if state.config.content_policy_enabled {
                        let terms = content::DangerousTermsConfig::default();
                        for note in &record.clinical_notes {
                            record_violations.extend(
                                content::check_content_policy(
                                    &note.text,
                                    &terms,
                                    record.patient_id.clone(),
                                ),
                            );
                        }
                    }

                    if state.config.plausibility_check_enabled {
                        record_violations.extend(plausibility::check_medical_plausibility(&record));
                    }

                    let has_errors = record_violations
                        .iter()
                        .any(|v| v.severity() == ViolationSeverity::Error);

                    if state.config.fail_on_error && has_errors {
                        batch_violations.reject(record.patient_id.clone());
                    } else {
                        if !record_violations.is_empty() {
                            batch_violations.add(record.patient_id.clone(), record_violations);
                        }
                        passed_records.push(record);
                    }
                }

                if state.config.distribution_check_enabled && !passed_records.is_empty() {
                    let dist_violations = distribution::check_distribution(
                        &passed_records,
                        &state.conditions_config,
                        state.config.distribution_tolerance,
                    );
                    if !dist_violations.is_empty() {
                        batch_violations.add("aggregate".to_string(), dist_violations);
                    }
                }

                if state.config.uniqueness_check_enabled {
                    let unique_violations = uniqueness::check_uniqueness(&passed_records);
                    if !unique_violations.is_empty() {
                        let dup_ids: Vec<String> = unique_violations
                            .iter()
                            .filter_map(|v| {
                                if let crate::domain::guardrail::ViolationType::Uniqueness { duplicate_id } =
                                    &v.violation_type
                                {
                                    Some(duplicate_id.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        batch_violations.rejected_ids.extend(dup_ids);
                        batch_violations.errors.extend(unique_violations);
                        passed_records
                            .retain(|r| !batch_violations.rejected_ids.contains(&r.patient_id));
                    }
                }

                let elapsed_ms = started.elapsed().as_millis() as u64;
                state.record_batch(&batch_violations, records_checked, elapsed_ms);

                if batch_violations.count > 0 {
                    tracing::info!(
                        actor = "GuardrailActor",
                        batch_id,
                        warnings = batch_violations.warnings.len(),
                        errors = batch_violations.errors.len(),
                        rejected = batch_violations.rejected_ids.len(),
                        duration_ms = elapsed_ms,
                        "Guardrail check complete"
                    );
                }

                state.downstream.cast(PipelineMsg::ClinicalNoteBatchGenerated {
                    job_id,
                    batch_id,
                    records: passed_records,
                })?;
            }
            PipelineMsg::Shutdown => {
                if let Err(e) = write_guardrail_report(&state.report, &state.output_dir).await {
                    tracing::error!(error = %e, "Failed to write guardrail report");
                }
                state.downstream.cast(PipelineMsg::Shutdown)?;
            }
            _ => {}
        }
        Ok(())
    }
}
