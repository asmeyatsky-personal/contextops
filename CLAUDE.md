# ContextOps™

The operational discipline for designing, versioning, governing, and continuously validating shared context across multi-agent AI systems.

## Architecture

ContextOps™ follows the Smeyatsky Labs architectural constitution (skill2026):
- **Hexagonal / Clean Architecture** — domain logic is independent of infrastructure
- **DDD Bounded Contexts** — ContextRegistry and ContextPipeline are separate contexts with explicit boundaries
- **MCP-Native** — all agent-facing interactions go through the MCP protocol layer
- **Parallelism-First** — independent operations fan out concurrently (DAG orchestration)
- **Immutable Domain Entities** — all state changes produce new instances + domain events

## Project Structure

```
crates/
  domain/           # Shared domain primitives (entities, value objects, events, ports)
  registry/          # M1 — Context Registry bounded context
  pipeline/          # M2 — Context Pipeline bounded context
  mcp-server/        # MCP protocol layer (JSON-RPC 2.0)
  cli/               # CLI application (contextops binary)
  common/            # Shared infrastructure adapters (in-memory implementations)
```

### Layer Separation (per bounded context)

```
domain/
  entities/          # Aggregate roots, entities
  value_objects/     # Immutable value types
  events/            # Domain events
  services/          # Domain services (pure business logic)
  ports/             # Interface definitions (Protocol traits)

application/
  commands/          # Write use cases (one class per use case)
  queries/           # Read use cases
  dtos/              # Data transfer objects

infrastructure/
  repositories/      # Port implementations (adapters)
  adapters/          # External service adapters
  container.rs       # Dependency injection / composition root
```

## Build & Test

```bash
cargo build              # Build all crates
cargo test               # Run all tests (51 tests)
cargo run -- --help      # CLI help
cargo run -- serve       # Start MCP server (stdio)
```

## CLI Usage

```bash
# Register a context artifact
contextops register -n "billing-rules" -s "org/finance" -t team \
  -f claude-md -o "finance-team" --file ./CLAUDE.md \
  -a "alice" -m "initial billing context"

# List artifacts
contextops list
contextops list --tier organisation
contextops list --namespace "org/finance"

# Search
contextops search "billing"

# Run pipeline
contextops pipeline run <artifact-uuid>

# Resolve inheritance chain
contextops resolve "org/finance/billing"
```

## MCP Server

The MCP server exposes ContextOps as tools and resources for AI agents:

**Tools** (write operations):
- `register_artifact` — register a new context artifact
- `create_version` — create a new version
- `deprecate_artifact` — deprecate an artifact
- `run_pipeline` — execute the context pipeline

**Resources** (read operations):
- `contextops://artifacts` — list all artifacts
- `contextops://artifacts/{id}` — get artifact detail
- `contextops://artifacts/{id}/content` — get raw content
- `contextops://resolve/{namespace}` — resolve inheritance chain
- `contextops://pipeline/runs` — list pipeline runs

## Key Conventions

- Domain layer has ZERO infrastructure dependencies
- All external dependencies are behind port interfaces (traits)
- Tests at every layer: domain (pure unit), application (mocked ports), integration (real adapters)
- Domain events collected on aggregates, dispatched after persistence
- Content-addressable storage with SHA-256 digests
- Three-tier context hierarchy: Organisation > Team > Project

## Stack

- **Rust** — core services (registry, pipeline, MCP server, CLI)
- **Python** — drift detection, security scanning (Phase 2)
- **React/TypeScript** — Governance Console (Phase 3)
- **GCP** — Firestore, BigQuery, Pub/Sub, Cloud Run (production adapters)
