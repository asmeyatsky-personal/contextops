use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{PipelineRun, RunStatus, StageResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunDto {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_name: String,
    pub artifact_id: Uuid,
    pub trigger: String,
    pub status: RunStatus,
    pub stage_count: usize,
    pub stages_completed: usize,
    pub stages_failed: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<&PipelineRun> for PipelineRunDto {
    fn from(run: &PipelineRun) -> Self {
        let stages = run.stage_results();
        Self {
            id: run.id(),
            pipeline_id: run.pipeline_id(),
            pipeline_name: run.pipeline_name().to_string(),
            artifact_id: run.artifact_id(),
            trigger: run.trigger().to_string(),
            status: run.status(),
            stage_count: stages.len(),
            stages_completed: stages
                .iter()
                .filter(|s| s.status == crate::domain::entities::StageStatus::Succeeded)
                .count(),
            stages_failed: stages
                .iter()
                .filter(|s| s.status == crate::domain::entities::StageStatus::Failed)
                .count(),
            started_at: run.started_at(),
            completed_at: run.completed_at(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunDetailDto {
    #[serde(flatten)]
    pub summary: PipelineRunDto,
    pub stages: Vec<StageResultDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResultDto {
    pub stage_name: String,
    pub status: String,
    pub duration_ms: u64,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

impl From<&StageResult> for StageResultDto {
    fn from(r: &StageResult) -> Self {
        Self {
            stage_name: r.stage_name.clone(),
            status: format!("{:?}", r.status),
            duration_ms: r.duration_ms,
            output: r.output.clone(),
            error: r.error.clone(),
        }
    }
}

impl From<&PipelineRun> for PipelineRunDetailDto {
    fn from(run: &PipelineRun) -> Self {
        Self {
            summary: PipelineRunDto::from(run),
            stages: run.stage_results().iter().map(StageResultDto::from).collect(),
        }
    }
}
