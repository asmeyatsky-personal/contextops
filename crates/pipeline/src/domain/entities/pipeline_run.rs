use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use contextops_domain::events::PipelineEvent;

/// The status of a pipeline run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    RolledBack,
    Cancelled,
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled-back"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Status of a single stage within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// Result of executing a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_name: String,
    pub status: StageStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

/// A pipeline run — immutable execution state for a pipeline.
///
/// Tracks which stages have completed, their results, and the
/// overall run status. Domain events are collected for each
/// state transition.
///
/// Immutable domain model: all state changes return new instances
/// via move semantics (mut self -> Self).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    id: Uuid,
    pipeline_id: Uuid,
    pipeline_name: String,
    artifact_id: Uuid,
    trigger: String,
    status: RunStatus,
    stage_results: Vec<StageResult>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    domain_events: Vec<PipelineEvent>,
}

impl PipelineRun {
    pub fn start(
        pipeline_id: Uuid,
        pipeline_name: String,
        artifact_id: Uuid,
        trigger: String,
    ) -> Self {
        let id = Uuid::new_v4();
        let event = PipelineEvent::PipelineRunStarted {
            run_id: id,
            pipeline_id,
            trigger: trigger.clone(),
        };

        Self {
            id,
            pipeline_id,
            pipeline_name,
            artifact_id,
            trigger,
            status: RunStatus::Running,
            stage_results: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            domain_events: vec![event],
        }
    }

    /// Record a stage completion. Returns a new instance.
    pub fn record_stage_success(
        mut self,
        stage_name: String,
        duration_ms: u64,
        output: serde_json::Value,
    ) -> Self {
        self.stage_results.push(StageResult {
            stage_name: stage_name.clone(),
            status: StageStatus::Succeeded,
            started_at: None,
            completed_at: Some(Utc::now()),
            duration_ms,
            output,
            error: None,
        });
        self.domain_events.push(PipelineEvent::StageCompleted {
            run_id: self.id,
            stage_name,
            status: "succeeded".into(),
            duration_ms,
        });
        self
    }

    /// Record a stage failure. Returns a new instance.
    pub fn record_stage_failure(
        mut self,
        stage_name: String,
        duration_ms: u64,
        error: String,
    ) -> Self {
        self.stage_results.push(StageResult {
            stage_name: stage_name.clone(),
            status: StageStatus::Failed,
            started_at: None,
            completed_at: Some(Utc::now()),
            duration_ms,
            output: serde_json::Value::Null,
            error: Some(error.clone()),
        });
        self.domain_events.push(PipelineEvent::StageFailed {
            run_id: self.id,
            stage_name,
            error,
        });
        self
    }

    /// Mark the run as completed successfully. Returns a new instance.
    pub fn complete(mut self) -> Self {
        self.status = RunStatus::Succeeded;
        self.completed_at = Some(Utc::now());
        let total_ms = self.stage_results.iter().map(|r| r.duration_ms).sum();
        self.domain_events.push(PipelineEvent::PipelineRunCompleted {
            run_id: self.id,
            pipeline_id: self.pipeline_id,
            status: "succeeded".into(),
            total_duration_ms: total_ms,
        });
        self
    }

    /// Mark the run as failed. Returns a new instance.
    pub fn fail(mut self) -> Self {
        self.status = RunStatus::Failed;
        self.completed_at = Some(Utc::now());
        let total_ms = self.stage_results.iter().map(|r| r.duration_ms).sum();
        self.domain_events.push(PipelineEvent::PipelineRunCompleted {
            run_id: self.id,
            pipeline_id: self.pipeline_id,
            status: "failed".into(),
            total_duration_ms: total_ms,
        });
        self
    }

    /// Trigger a rollback. Returns a new instance.
    pub fn rollback(mut self, reason: String, previous_vaid: Uuid) -> Self {
        self.status = RunStatus::RolledBack;
        self.completed_at = Some(Utc::now());
        self.domain_events.push(PipelineEvent::RollbackTriggered {
            run_id: self.id,
            reason,
            previous_vaid,
        });
        self
    }

    // --- Accessors ---

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn pipeline_id(&self) -> Uuid {
        self.pipeline_id
    }

    pub fn pipeline_name(&self) -> &str {
        &self.pipeline_name
    }

    pub fn artifact_id(&self) -> Uuid {
        self.artifact_id
    }

    pub fn trigger(&self) -> &str {
        &self.trigger
    }

    pub fn status(&self) -> RunStatus {
        self.status
    }

    pub fn stage_results(&self) -> &[StageResult] {
        &self.stage_results
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Drain and return all pending domain events.
    pub fn take_events(mut self) -> (Self, Vec<PipelineEvent>) {
        let events = std::mem::take(&mut self.domain_events);
        (self, events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_creates_running_run() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test-pipeline".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        assert_eq!(run.status(), RunStatus::Running);
        assert!(run.completed_at().is_none());
    }

    #[test]
    fn record_stage_success_returns_new_instance() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        let run = run.record_stage_success("validate".into(), 150, serde_json::json!({"ok": true}));
        assert_eq!(run.stage_results().len(), 1);
        assert_eq!(run.stage_results()[0].status, StageStatus::Succeeded);
    }

    #[test]
    fn complete_returns_succeeded_instance() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        let run = run.record_stage_success("validate".into(), 100, serde_json::Value::Null);
        let run = run.complete();
        assert_eq!(run.status(), RunStatus::Succeeded);
        assert!(run.completed_at().is_some());
    }

    #[test]
    fn fail_returns_failed_instance() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        let run = run.record_stage_failure("validate".into(), 50, "schema error".into());
        let run = run.fail();
        assert_eq!(run.status(), RunStatus::Failed);
    }

    #[test]
    fn rollback_returns_rolled_back_instance() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        let run = run.rollback("drift threshold breached".into(), Uuid::new_v4());
        assert_eq!(run.status(), RunStatus::RolledBack);
        assert!(run.completed_at().is_some());
    }

    #[test]
    fn take_events_returns_all_collected_events() {
        let run = PipelineRun::start(
            Uuid::new_v4(),
            "test".into(),
            Uuid::new_v4(),
            "manual".into(),
        );
        let run = run.record_stage_success("validate".into(), 100, serde_json::Value::Null);
        let run = run.complete();
        let (_run, events) = run.take_events();
        assert_eq!(events.len(), 3); // started + stage_completed + run_completed
    }
}
