//! Guardrail report JSON writer.

use crate::domain::guardrail::GuardrailReport;
use std::path::Path;

pub async fn write_guardrail_report(
    report: &GuardrailReport,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let report_path = output_dir.join("guardrail_report.json");
    if let Some(parent) = report_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(report)?;
    tokio::fs::write(&report_path, json).await?;

    tracing::info!("Guardrail report written to {:?}", report_path);
    Ok(())
}
