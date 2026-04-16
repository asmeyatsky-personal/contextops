/// ContextOps™ CLI
///
/// Command-line interface for context management.
/// Runs in embedded mode with in-memory adapters for local development.
/// Production mode connects to a remote MCP server.

use clap::{Parser, Subcommand};
use contextops_domain::entities::ContextFormat;
use contextops_domain::value_objects::ContextTier;
use contextops_pipeline::infrastructure::container::PipelineContainer;
use contextops_registry::infrastructure::container::RegistryContainer;

#[derive(Parser)]
#[command(
    name = "contextops",
    about = "ContextOps™ — Context management for multi-agent AI systems",
    version,
    long_about = "The operational discipline for designing, versioning, governing, and \
                  continuously validating shared context across multi-agent AI systems."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new context artifact
    Register {
        /// Artifact name
        #[arg(short, long)]
        name: String,

        /// Namespace (e.g., 'org/finance')
        #[arg(short = 's', long)]
        namespace: String,

        /// Context tier: organisation, team, or project
        #[arg(short, long, default_value = "project")]
        tier: String,

        /// Format: claude-md, json, yaml, plain-text
        #[arg(long, default_value = "claude-md")]
        format: String,

        /// Owner
        #[arg(short, long)]
        owner: String,

        /// Path to the context file
        #[arg(short, long)]
        file: String,

        /// Author name
        #[arg(short, long)]
        author: String,

        /// Version message
        #[arg(short, long)]
        message: String,
    },

    /// List context artifacts
    List {
        /// Filter by tier
        #[arg(short, long)]
        tier: Option<String>,

        /// Filter by namespace
        #[arg(short = 's', long)]
        namespace: Option<String>,
    },

    /// Get artifact details
    Get {
        /// Artifact ID (UUID)
        id: String,
    },

    /// Create a new version of an artifact
    Version {
        /// Artifact ID (UUID)
        #[arg(short, long)]
        artifact_id: String,

        /// Path to the new content file
        #[arg(short = 'f', long)]
        file: String,

        /// Author name
        #[arg(short, long)]
        author: String,

        /// Version message
        #[arg(short, long)]
        message: String,
    },

    /// Search context artifacts
    Search {
        /// Search query
        query: String,

        /// Filter by tier
        #[arg(short, long)]
        tier: Option<String>,
    },

    /// Run a context pipeline against an artifact
    Pipeline {
        #[command(subcommand)]
        command: PipelineCommands,
    },

    /// Resolve the effective context for a namespace
    Resolve {
        /// Namespace to resolve
        namespace: String,
    },

    /// Start the MCP server (stdio transport)
    Serve,
}

#[derive(Subcommand)]
enum PipelineCommands {
    /// Run the pipeline against an artifact
    Run {
        /// Artifact ID (UUID)
        artifact_id: String,

        /// Trigger description
        #[arg(short, long, default_value = "cli")]
        trigger: String,
    },

    /// Get pipeline run status
    Status {
        /// Run ID (UUID)
        run_id: String,
    },

    /// List recent pipeline runs
    Runs {
        /// Number of runs to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    // Composition root — wire up in-memory adapters for embedded mode
    let registry = RegistryContainer::in_memory();
    let pipeline = PipelineContainer::in_memory(registry.repository.clone());

    match cli.command {
        Commands::Register {
            name,
            namespace,
            tier,
            format,
            owner,
            file,
            author,
            message,
        } => {
            let content = std::fs::read(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file, e))?;

            let tier = parse_tier(&tier)?;
            let format = parse_format(&format)?;

            let input = contextops_registry::application::commands::register_artifact::RegisterArtifactInput {
                name, namespace, tier, format, owner, content, author, message,
            };

            let result = registry.register_artifact.execute(input).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::List { tier, namespace } => {
            let results = if let Some(tier_str) = tier {
                let tier = parse_tier(&tier_str)?;
                registry.list_artifacts.by_tier(tier).await?
            } else if let Some(ns) = namespace {
                registry.list_artifacts.by_namespace(&ns).await?
            } else {
                registry.list_artifacts.all(0, 100).await?
            };

            if results.is_empty() {
                println!("No artifacts found.");
            } else {
                println!("{}", serde_json::to_string_pretty(&results)?);
            }
        }

        Commands::Get { id } => {
            let uuid: uuid::Uuid = id.parse().map_err(|_| anyhow::anyhow!("Invalid UUID"))?;
            let result = registry.get_artifact.by_id(uuid).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::Version {
            artifact_id,
            file,
            author,
            message,
        } => {
            let uuid: uuid::Uuid = artifact_id
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid UUID"))?;
            let content = std::fs::read(&file)
                .map_err(|e| anyhow::anyhow!("Failed to read file '{}': {}", file, e))?;

            let input = contextops_registry::application::commands::create_version::CreateVersionInput {
                artifact_id: uuid,
                content,
                author,
                message,
                commit_sha: None,
            };

            let result = registry.create_version.execute(input).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        Commands::Search { query, tier } => {
            let tier_filter = tier.as_deref().map(parse_tier).transpose()?;
            let results = registry
                .search_artifacts
                .search(&query, tier_filter, 20)
                .await?;

            if results.is_empty() {
                println!("No results found.");
            } else {
                for result in results {
                    println!(
                        "[{:.2}] {} — {} ({})",
                        result.score, result.name, result.namespace, result.tier
                    );
                    println!("  {}", result.snippet);
                }
            }
        }

        Commands::Pipeline { command } => match command {
            PipelineCommands::Run {
                artifact_id,
                trigger,
            } => {
                let uuid: uuid::Uuid = artifact_id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid UUID"))?;

                let input = contextops_pipeline::application::commands::RunPipelineInput {
                    pipeline_id: None,
                    artifact_id: uuid,
                    trigger,
                };

                let result = pipeline.run_pipeline.execute(input).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }

            PipelineCommands::Status { run_id } => {
                let uuid: uuid::Uuid = run_id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid UUID"))?;
                let result = pipeline.get_pipeline_run.by_id(uuid).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }

            PipelineCommands::Runs { limit } => {
                let results = pipeline.get_pipeline_run.recent(limit).await?;
                if results.is_empty() {
                    println!("No pipeline runs found.");
                } else {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
            }
        },

        Commands::Resolve { namespace } => {
            let resolved = registry.resolve_context.resolve(&namespace).await?;
            println!("Resolved context for namespace '{namespace}':");
            println!("  Layers: {}", resolved.layers.len());
            println!("  Composite hash: {}", resolved.composite_hash);
            println!("  Conflicts: {}", resolved.conflicts.len());
            for layer in &resolved.layers {
                println!(
                    "  - [{}] {} ({})",
                    layer.tier, layer.artifact_name, layer.content_hash
                );
            }
        }

        Commands::Serve => {
            eprintln!("ContextOps™ MCP Server starting on stdio...");
            eprintln!("Protocol: JSON-RPC 2.0 over stdio");

            // Read JSON-RPC requests from stdin, write responses to stdout
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

            let mcp_server = contextops_mcp_server::server::ContextOpsMcpServer::new(
                registry.register_artifact,
                registry.create_version,
                registry.deprecate_artifact,
                registry.get_artifact,
                registry.list_artifacts,
                registry.search_artifacts,
                registry.resolve_context,
                pipeline.run_pipeline,
                pipeline.get_pipeline_run,
            );

            let stdin = tokio::io::stdin();
            let mut stdout = tokio::io::stdout();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                match serde_json::from_str::<contextops_mcp_server::protocol::JsonRpcRequest>(&line)
                {
                    Ok(request) => {
                        let response = mcp_server.handle_request(request).await;
                        let response_json = serde_json::to_string(&response).unwrap();
                        stdout
                            .write_all(response_json.as_bytes())
                            .await
                            .ok();
                        stdout.write_all(b"\n").await.ok();
                        stdout.flush().await.ok();
                    }
                    Err(e) => {
                        let error_response = contextops_mcp_server::protocol::JsonRpcResponse::error(
                            serde_json::Value::Null,
                            -32700,
                            format!("Parse error: {e}"),
                        );
                        let response_json = serde_json::to_string(&error_response).unwrap();
                        stdout
                            .write_all(response_json.as_bytes())
                            .await
                            .ok();
                        stdout.write_all(b"\n").await.ok();
                        stdout.flush().await.ok();
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_tier(s: &str) -> anyhow::Result<ContextTier> {
    match s.to_lowercase().as_str() {
        "organisation" | "org" | "1" => Ok(ContextTier::Organisation),
        "team" | "2" => Ok(ContextTier::Team),
        "project" | "3" => Ok(ContextTier::Project),
        _ => Err(anyhow::anyhow!(
            "Invalid tier '{}'. Use: organisation, team, or project",
            s
        )),
    }
}

fn parse_format(s: &str) -> anyhow::Result<ContextFormat> {
    match s.to_lowercase().as_str() {
        "claude-md" | "md" => Ok(ContextFormat::ClaudeMd),
        "json" => Ok(ContextFormat::Json),
        "yaml" | "yml" => Ok(ContextFormat::Yaml),
        "plain-text" | "txt" => Ok(ContextFormat::PlainText),
        "prompt-template" | "prompt" => Ok(ContextFormat::PromptTemplate),
        _ => Err(anyhow::anyhow!(
            "Invalid format '{}'. Use: claude-md, json, yaml, plain-text, or prompt-template",
            s
        )),
    }
}
