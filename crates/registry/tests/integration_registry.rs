/// Integration tests for the Context Registry bounded context.
///
/// These tests verify the full application layer — use cases
/// orchestrating domain objects through in-memory adapters.

use contextops_domain::entities::ContextFormat;
use contextops_domain::value_objects::ContextTier;
use contextops_registry::application::commands::register_artifact::RegisterArtifactInput;
use contextops_registry::application::commands::create_version::CreateVersionInput;
use contextops_registry::application::commands::deprecate_artifact::DeprecateArtifactInput;
use contextops_registry::infrastructure::container::RegistryContainer;

fn setup() -> RegistryContainer {
    RegistryContainer::in_memory()
}

#[tokio::test]
async fn register_and_retrieve_artifact() {
    let container = setup();

    let input = RegisterArtifactInput {
        name: "billing-rules".into(),
        namespace: "org/finance".into(),
        tier: ContextTier::Team,
        format: ContextFormat::ClaudeMd,
        owner: "finance-team".into(),
        content: b"# Billing Rules\n\nAll invoices net 30.".to_vec(),
        author: "alice".into(),
        message: "initial billing context".into(),
    };

    let result = container.register_artifact.execute(input).await.unwrap();
    assert_eq!(result.name, "billing-rules");
    assert_eq!(result.current_version, 1);
    assert!(!result.deprecated);

    // Retrieve by ID
    let detail = container.get_artifact.by_id(result.id).await.unwrap();
    assert_eq!(detail.artifact.name, "billing-rules");
    assert_eq!(detail.versions.len(), 1);
}

#[tokio::test]
async fn create_version_updates_artifact() {
    let container = setup();

    let register_input = RegisterArtifactInput {
        name: "api-config".into(),
        namespace: "org/engineering".into(),
        tier: ContextTier::Project,
        format: ContextFormat::Json,
        owner: "platform".into(),
        content: br#"{"timeout": 30}"#.to_vec(),
        author: "bob".into(),
        message: "initial config".into(),
    };

    let artifact = container.register_artifact.execute(register_input).await.unwrap();

    let version_input = CreateVersionInput {
        artifact_id: artifact.id,
        content: br#"{"timeout": 60, "retries": 3}"#.to_vec(),
        author: "carol".into(),
        message: "increased timeout and added retries".into(),
        commit_sha: Some("abc123def".into()),
    };

    let updated = container.create_version.execute(version_input).await.unwrap();
    assert_eq!(updated.current_version, 2);

    let detail = container.get_artifact.by_id(artifact.id).await.unwrap();
    assert_eq!(detail.versions.len(), 2);
    assert_eq!(detail.versions[1].author, "carol");
}

#[tokio::test]
async fn deprecate_prevents_new_versions() {
    let container = setup();

    let register_input = RegisterArtifactInput {
        name: "old-rules".into(),
        namespace: "org/legacy".into(),
        tier: ContextTier::Project,
        format: ContextFormat::PlainText,
        owner: "legacy-team".into(),
        content: b"deprecated rules".to_vec(),
        author: "dave".into(),
        message: "legacy context".into(),
    };

    let artifact = container.register_artifact.execute(register_input).await.unwrap();

    let deprecate_input = DeprecateArtifactInput {
        artifact_id: artifact.id,
        reason: "replaced by new-rules".into(),
    };

    let deprecated = container.deprecate_artifact.execute(deprecate_input).await.unwrap();
    assert!(deprecated.deprecated);

    // Attempting to create a new version should fail
    let version_input = CreateVersionInput {
        artifact_id: artifact.id,
        content: b"new content".to_vec(),
        author: "eve".into(),
        message: "should fail".into(),
        commit_sha: None,
    };

    let result = container.create_version.execute(version_input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn duplicate_registration_is_rejected() {
    let container = setup();

    let input = RegisterArtifactInput {
        name: "unique-name".into(),
        namespace: "org/test".into(),
        tier: ContextTier::Project,
        format: ContextFormat::ClaudeMd,
        owner: "test".into(),
        content: b"content".to_vec(),
        author: "author".into(),
        message: "first".into(),
    };

    container.register_artifact.execute(input).await.unwrap();

    let duplicate = RegisterArtifactInput {
        name: "unique-name".into(),
        namespace: "org/test".into(),
        tier: ContextTier::Project,
        format: ContextFormat::ClaudeMd,
        owner: "test".into(),
        content: b"different content".to_vec(),
        author: "author".into(),
        message: "duplicate".into(),
    };

    let result = container.register_artifact.execute(duplicate).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_artifacts_by_tier() {
    let container = setup();

    // Register artifacts across tiers
    for (name, tier) in [
        ("org-rules", ContextTier::Organisation),
        ("team-config", ContextTier::Team),
        ("agent-setup", ContextTier::Project),
    ] {
        let input = RegisterArtifactInput {
            name: name.into(),
            namespace: "org/test".into(),
            tier,
            format: ContextFormat::ClaudeMd,
            owner: "test".into(),
            content: format!("# {name}").into_bytes(),
            author: "author".into(),
            message: "msg".into(),
        };
        container.register_artifact.execute(input).await.unwrap();
    }

    let org_artifacts = container
        .list_artifacts
        .by_tier(ContextTier::Organisation)
        .await
        .unwrap();
    assert_eq!(org_artifacts.len(), 1);
    assert_eq!(org_artifacts[0].name, "org-rules");

    let all = container.list_artifacts.all(0, 100).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn search_finds_artifacts_by_content() {
    let container = setup();

    let input = RegisterArtifactInput {
        name: "security-policy".into(),
        namespace: "org/security".into(),
        tier: ContextTier::Organisation,
        format: ContextFormat::ClaudeMd,
        owner: "ciso".into(),
        content: b"# Security Policy\n\nAll agents must validate VAID before executing.".to_vec(),
        author: "security-team".into(),
        message: "initial security context".into(),
    };

    container.register_artifact.execute(input).await.unwrap();

    let results = container
        .search_artifacts
        .search("VAID", None, 10)
        .await
        .unwrap();

    assert!(!results.is_empty());
    assert_eq!(results[0].name, "security-policy");
}

#[tokio::test]
async fn resolve_context_builds_inheritance_chain() {
    let container = setup();

    // Register artifacts at different tiers
    for (name, tier) in [
        ("company-baseline", ContextTier::Organisation),
        ("team-conventions", ContextTier::Team),
    ] {
        let input = RegisterArtifactInput {
            name: name.into(),
            namespace: "org/engineering".into(),
            tier,
            format: ContextFormat::ClaudeMd,
            owner: "test".into(),
            content: format!("# {name}\nContent here.").into_bytes(),
            author: "author".into(),
            message: "msg".into(),
        };
        container.register_artifact.execute(input).await.unwrap();
    }

    let resolved = container
        .resolve_context
        .resolve("org/engineering")
        .await
        .unwrap();

    assert!(resolved.layers.len() >= 1);
    assert!(resolved.conflicts.is_empty());
}

#[tokio::test]
async fn content_retrieval_returns_stored_bytes() {
    let container = setup();

    let content = b"# Exact Content\n\nThis should be retrievable byte-for-byte.";

    let input = RegisterArtifactInput {
        name: "content-test".into(),
        namespace: "org/test".into(),
        tier: ContextTier::Project,
        format: ContextFormat::ClaudeMd,
        owner: "test".into(),
        content: content.to_vec(),
        author: "author".into(),
        message: "content test".into(),
    };

    let artifact = container.register_artifact.execute(input).await.unwrap();

    let retrieved = container
        .get_artifact
        .content(artifact.id, None)
        .await
        .unwrap();

    assert_eq!(retrieved, content);
}
