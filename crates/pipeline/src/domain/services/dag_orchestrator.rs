use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use contextops_domain::errors::DomainError;

use crate::domain::entities::{Pipeline, PipelineRun, StageKind};
use crate::domain::ports::stage_executor::{StageContext, StageExecutorPort};

/// DAG-based pipeline orchestrator.
///
/// Executes pipeline stages respecting dependency order,
/// parallelizing independent stages automatically.
/// This is Rule 7 (Parallel-Safe Orchestration) from skill2026.
pub struct DagOrchestrator {
    executors: HashMap<StageKind, Arc<dyn StageExecutorPort>>,
}

impl DagOrchestrator {
    pub fn new(executors: Vec<Arc<dyn StageExecutorPort>>) -> Self {
        let executor_map = executors
            .into_iter()
            .map(|e| (e.stage_kind(), e))
            .collect();
        Self {
            executors: executor_map,
        }
    }

    /// Execute a pipeline, running independent stages concurrently.
    pub async fn execute(
        &self,
        pipeline: &Pipeline,
        stage_context: StageContext,
    ) -> Result<PipelineRun, DomainError> {
        let levels = pipeline.topological_sort()?;
        let mut run = PipelineRun::start(
            pipeline.id(),
            pipeline.name().to_string(),
            stage_context.artifact_id,
            "orchestrator".into(),
        );

        let mut accumulated_results: HashMap<String, serde_json::Value> = HashMap::new();

        for level in levels {
            // All stages at the same level run concurrently
            let mut futures = Vec::new();

            for stage in &level {
                let executor = self.executors.get(&stage.kind).ok_or_else(|| {
                    DomainError::PipelineStageFailed {
                        stage_name: stage.name.clone(),
                        reason: format!("no executor registered for stage kind {:?}", stage.kind),
                    }
                })?;

                let ctx = StageContext {
                    previous_results: accumulated_results.clone(),
                    ..stage_context.clone()
                };

                let executor = executor.clone();
                let stage_name = stage.name.clone();
                let required = stage.required;

                futures.push(tokio::spawn(async move {
                    let start = Instant::now();
                    let result = executor.execute(&ctx).await;
                    let duration_ms = start.elapsed().as_millis() as u64;
                    (stage_name, required, result, duration_ms)
                }));
            }

            // Await all parallel stages
            let results = futures::future::join_all(futures).await;
            let mut level_failed = false;

            for join_result in results {
                let (stage_name, required, exec_result, duration_ms) =
                    join_result.map_err(|e| DomainError::PipelineStageFailed {
                        stage_name: "join".into(),
                        reason: e.to_string(),
                    })?;

                match exec_result {
                    Ok(output) => {
                        accumulated_results.insert(stage_name.clone(), output.clone());
                        run.record_stage_success(stage_name, duration_ms, output);
                    }
                    Err(err) => {
                        run.record_stage_failure(stage_name.clone(), duration_ms, err.to_string());
                        if required {
                            level_failed = true;
                        }
                    }
                }
            }

            if level_failed {
                run.fail();
                return Ok(run);
            }
        }

        run.complete();
        Ok(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::PipelineStage;
    use crate::domain::ports::stage_executor::StageExecutorError;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    struct MockExecutor {
        kind: StageKind,
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl StageExecutorPort for MockExecutor {
        fn stage_kind(&self) -> StageKind {
            self.kind
        }

        async fn execute(&self, _ctx: &StageContext) -> Result<serde_json::Value, StageExecutorError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!({"status": "ok"}))
        }
    }

    struct FailingExecutor {
        kind: StageKind,
    }

    #[async_trait]
    impl StageExecutorPort for FailingExecutor {
        fn stage_kind(&self) -> StageKind {
            self.kind
        }

        async fn execute(&self, _ctx: &StageContext) -> Result<serde_json::Value, StageExecutorError> {
            Err(StageExecutorError::QualityGateFailed {
                violations: vec!["test failure".into()],
            })
        }
    }

    fn make_context() -> StageContext {
        StageContext {
            artifact_id: Uuid::new_v4(),
            content: b"test content".to_vec(),
            content_hash: "abc123".into(),
            namespace: "test".into(),
            tier: "project".into(),
            previous_results: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn orchestrator_executes_all_stages() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let pipeline = Pipeline::new(
            "test".into(),
            "".into(),
            vec![
                PipelineStage {
                    name: "validate".into(),
                    kind: StageKind::Validate,
                    depends_on: vec![],
                    timeout_seconds: 60,
                    required: true,
                },
                PipelineStage {
                    name: "scan".into(),
                    kind: StageKind::SecurityScan,
                    depends_on: vec!["validate".into()],
                    timeout_seconds: 60,
                    required: true,
                },
            ],
            "user".into(),
        )
        .unwrap();

        let orchestrator = DagOrchestrator::new(vec![
            Arc::new(MockExecutor {
                kind: StageKind::Validate,
                call_count: call_count.clone(),
            }),
            Arc::new(MockExecutor {
                kind: StageKind::SecurityScan,
                call_count: call_count.clone(),
            }),
        ]);

        let run = orchestrator.execute(&pipeline, make_context()).await.unwrap();
        assert_eq!(run.status(), crate::domain::entities::RunStatus::Succeeded);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn orchestrator_fails_on_required_stage_failure() {
        let pipeline = Pipeline::new(
            "test".into(),
            "".into(),
            vec![PipelineStage {
                name: "validate".into(),
                kind: StageKind::Validate,
                depends_on: vec![],
                timeout_seconds: 60,
                required: true,
            }],
            "user".into(),
        )
        .unwrap();

        let orchestrator = DagOrchestrator::new(vec![Arc::new(FailingExecutor {
            kind: StageKind::Validate,
        })]);

        let run = orchestrator.execute(&pipeline, make_context()).await.unwrap();
        assert_eq!(run.status(), crate::domain::entities::RunStatus::Failed);
    }
}
