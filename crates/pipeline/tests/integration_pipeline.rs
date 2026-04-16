/// Integration tests for the Context Pipeline bounded context.
///
/// Tests the DAG-based pipeline execution, stage ordering,
/// and parallel execution of independent stages.

use contextops_domain::entities::ContextFormat;
use contextops_domain::value_objects::ContextTier;
use contextops_pipeline::application::commands::RunPipelineInput;
use contextops_pipeline::domain::entities::RunStatus;
use contextops_pipeline::infrastructure::container::PipelineContainer;
use contextops_registry::application::commands::register_artifact::RegisterArtifactInput;
use contextops_registry::infrastructure::container::RegistryContainer;

fn setup() -> (RegistryContainer, PipelineContainer) {
    let registry = RegistryContainer::in_memory();
    let pipeline = PipelineContainer::in_memory(registry.repository.clone());
    (registry, pipeline)
}

#[tokio::test]
async fn run_standard_pipeline_against_valid_artifact() {
    let (registry, pipeline) = setup();

    // Register an artifact
    let input = RegisterArtifactInput {
        name: "test-context".into(),
        namespace: "org/test".into(),
        tier: ContextTier::Project,
        format: ContextFormat::ClaudeMd,
        owner: "test".into(),
        content: b"# Test Context\n\nThis is a test context file.".to_vec(),
        author: "tester".into(),
        message: "test artifact for pipeline".into(),
    };

    let artifact = registry.register_artifact.execute(input).await.unwrap();

    // Run the pipeline
    let run_input = RunPipelineInput {
        pipeline_id: None, // use standard pipeline
        artifact_id: artifact.id,
        trigger: "integration-test".into(),
    };

    let result = pipeline.run_pipeline.execute(run_input).await.unwrap();

    assert_eq!(result.summary.status, RunStatus::Succeeded);
    assert!(result.stages.len() >= 4); // validate, blast-radius, regression, security at minimum
    assert_eq!(result.summary.stages_failed, 0);
}

#[tokio::test]
async fn pipeline_detects_secrets_in_content() {
    let (registry, pipeline) = setup();

    // Register an artifact containing a potential secret
    let content = b"# Config\n\napi_key=sk_live_abcdef1234567890abcdef";

    let input = RegisterArtifactInput {
        name: "secret-test".into(),
        namespace: "org/test".into(),
        tier: ContextTier::Project,
        format: ContextFormat::ClaudeMd,
        owner: "test".into(),
        content: content.to_vec(),
        author: "tester".into(),
        message: "artifact with secret".into(),
    };

    let artifact = registry.register_artifact.execute(input).await.unwrap();

    let run_input = RunPipelineInput {
        pipeline_id: None,
        artifact_id: artifact.id,
        trigger: "security-test".into(),
    };

    let result = pipeline.run_pipeline.execute(run_input).await.unwrap();

    // The security scan should cause the pipeline to fail
    assert_eq!(result.summary.status, RunStatus::Failed);
    assert!(result.summary.stages_failed > 0);

    // Check that the security scan stage specifically failed
    let security_stage = result.stages.iter().find(|s| s.stage_name == "security-scan");
    assert!(security_stage.is_some());
    assert!(security_stage.unwrap().error.is_some());
}

#[tokio::test]
async fn pipeline_run_not_found_returns_error() {
    let (_, pipeline) = setup();

    let result = pipeline
        .get_pipeline_run
        .by_id(uuid::Uuid::new_v4())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn pipeline_for_nonexistent_artifact_fails() {
    let (_, pipeline) = setup();

    let run_input = RunPipelineInput {
        pipeline_id: None,
        artifact_id: uuid::Uuid::new_v4(),
        trigger: "test".into(),
    };

    let result = pipeline.run_pipeline.execute(run_input).await;
    assert!(result.is_err());
}
