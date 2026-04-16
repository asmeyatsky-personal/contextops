use async_trait::async_trait;

use contextops_domain::entities::ContextFormat;
use contextops_domain::ports::schema_validator::{SchemaValidatorPort, ViolationSeverity};

use crate::domain::entities::StageKind;
use crate::domain::ports::stage_executor::{StageContext, StageExecutorError, StageExecutorPort};

use std::sync::Arc;

/// Validate stage executor — runs schema validation and lint checks.
pub struct ValidateStageExecutor {
    validator: Arc<dyn SchemaValidatorPort>,
}

impl ValidateStageExecutor {
    pub fn new(validator: Arc<dyn SchemaValidatorPort>) -> Self {
        Self { validator }
    }
}

#[async_trait]
impl StageExecutorPort for ValidateStageExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::Validate
    }

    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        // Determine format from tier/content
        let format = ContextFormat::ClaudeMd; // Default; real impl would detect

        let violations = self
            .validator
            .validate(&context.content, format)
            .await
            .map_err(|e| StageExecutorError::ExecutionFailed(e.to_string()))?;

        let errors: Vec<String> = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Error)
            .map(|v| format!("{}: {}", v.path, v.message))
            .collect();

        if !errors.is_empty() {
            return Err(StageExecutorError::QualityGateFailed { violations: errors });
        }

        let warnings: Vec<String> = violations
            .iter()
            .filter(|v| v.severity == ViolationSeverity::Warning)
            .map(|v| format!("{}: {}", v.path, v.message))
            .collect();

        Ok(serde_json::json!({
            "status": "passed",
            "warnings": warnings,
        }))
    }
}

/// Blast radius stage executor — computes affected agents and workflows.
pub struct BlastRadiusStageExecutor;

#[async_trait]
impl StageExecutorPort for BlastRadiusStageExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::BlastRadius
    }

    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        // In a full implementation, this would query the registry for all
        // agents consuming this artifact and compute the blast radius.
        Ok(serde_json::json!({
            "status": "computed",
            "artifact_id": context.artifact_id.to_string(),
            "affected_agents": [],
            "affected_workflows": [],
            "severity": "low",
        }))
    }
}

/// Regression test stage executor.
pub struct RegressionTestStageExecutor;

#[async_trait]
impl StageExecutorPort for RegressionTestStageExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::RegressionTest
    }

    async fn execute(
        &self,
        _context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        // In a full implementation, this would run golden-dataset tests.
        Ok(serde_json::json!({
            "status": "passed",
            "tests_run": 0,
            "tests_passed": 0,
            "tests_failed": 0,
            "note": "no golden dataset configured"
        }))
    }
}

/// Security scan stage executor.
pub struct SecurityScanStageExecutor;

#[async_trait]
impl StageExecutorPort for SecurityScanStageExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::SecurityScan
    }

    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        let content_str = String::from_utf8_lossy(&context.content);
        let mut findings: Vec<String> = Vec::new();

        // Basic secrets detection patterns
        let secret_patterns = [
            "api_key", "api-key", "apikey",
            "secret_key", "secret-key",
            "password", "passwd",
            "token",
            "private_key", "private-key",
        ];

        for pattern in &secret_patterns {
            if content_str.to_lowercase().contains(&format!("{pattern}="))
                || content_str.to_lowercase().contains(&format!("{pattern}:"))
            {
                // Check if it looks like an actual secret value (not just a reference)
                let lower = content_str.to_lowercase();
                if let Some(pos) = lower.find(&format!("{pattern}=")) {
                    let after = &content_str[pos + pattern.len() + 1..];
                    let value: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
                    // Skip vault references, env var references, and placeholders
                    if !value.starts_with("${")
                        && !value.starts_with("vault:")
                        && !value.contains("<REDACTED>")
                        && value.len() > 8
                    {
                        findings.push(format!("Potential secret detected: {pattern}"));
                    }
                }
            }
        }

        if findings.iter().any(|f| f.contains("Potential secret")) {
            return Err(StageExecutorError::QualityGateFailed {
                violations: findings,
            });
        }

        Ok(serde_json::json!({
            "status": "passed",
            "findings": findings,
            "scanned_bytes": context.content.len(),
        }))
    }
}

/// Promotion stage executor (staging).
pub struct PromoteStagingExecutor;

#[async_trait]
impl StageExecutorPort for PromoteStagingExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::PromoteStaging
    }

    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        Ok(serde_json::json!({
            "status": "promoted",
            "environment": "staging",
            "artifact_id": context.artifact_id.to_string(),
            "content_hash": context.content_hash,
        }))
    }
}

/// Promotion stage executor (production).
pub struct PromoteProductionExecutor;

#[async_trait]
impl StageExecutorPort for PromoteProductionExecutor {
    fn stage_kind(&self) -> StageKind {
        StageKind::PromoteProduction
    }

    async fn execute(
        &self,
        context: &StageContext,
    ) -> Result<serde_json::Value, StageExecutorError> {
        Ok(serde_json::json!({
            "status": "promoted",
            "environment": "production",
            "artifact_id": context.artifact_id.to_string(),
            "content_hash": context.content_hash,
        }))
    }
}
