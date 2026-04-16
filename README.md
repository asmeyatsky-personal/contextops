# ContextOps™

**The operational discipline for designing, versioning, governing, and continuously validating shared context across multi-agent AI systems.**

ContextOps™ slots into the enterprise toolchain alongside DevOps, MLOps, and FinOps — a new -Ops category for a new operational risk: **context debt**.

---

## The Problem

Every organisation deploying multi-agent AI systems is accumulating context debt at pace. CLAUDE.md files proliferate, skill libraries diverge, project-level context contradicts company-level policy, and nobody owns the coherence layer.

The industry recognises two AI failure categories: **model failures** and **prompt failures**. ContextOps™ establishes the critical third:

| Failure Type | Description | Existing Tooling |
|---|---|---|
| Model Failure | The model produces wrong output given correct inputs | Evals, benchmarks, fine-tuning |
| Prompt Failure | The prompt is poorly structured or ambiguous | Prompt engineering, red-teaming |
| **Context Failure** | The context is stale, contradictory, missing, or unauthorised — causing correct model + correct prompt to produce wrong output | **Nothing. This is the gap ContextOps™ closes.** |

## Architecture

ContextOps™ is built to the Smeyatsky Labs architectural constitution:

- **Hexagonal / Clean Architecture** — domain logic is independent of infrastructure
- **DDD Bounded Contexts** — ContextRegistry and ContextPipeline are separate contexts with explicit anti-corruption layers
- **MCP-Native** — all agent-facing interactions go through the Model Context Protocol
- **Parallelism-First** — independent operations fan out concurrently via DAG orchestration
- **Immutable Domain Entities** — all state changes produce new instances and domain events

### Context Hierarchy

```
TIER 1 — ORGANISATION          Always true for every agent
  Company values · Security policies · Compliance rules

TIER 2 — TEAM / DOMAIN         True for a business function
  Team workflows · Domain terminology · Conventions

TIER 3 — PROJECT / AGENT       True for a specific agent
  CLAUDE.md · Skill files · Task instructions
```

Lower tiers extend but **never override** higher-tier compliance constraints.

## Product Modules

| Module | Description | Phase |
|--------|-------------|-------|
| **M1 — Context Registry** | Canonical store for all versioned context artifacts. Content-addressable, hierarchy-aware, VAID-binding. | Phase 1 ✅ |
| **M2 — Context Pipeline** | DAG-based CI/CD for context — validate, blast radius, regression test, security scan, promote, rollback. | Phase 1 ✅ |
| **M3 — Drift Detector** | MLOps-grade behavioural monitoring — detects agent drift from versioned baseline. | Phase 2 |
| **M4 — Security Gateway** | DevSecOps layer — RBAC, secrets isolation, prompt injection analysis, compliance enforcement. | Phase 2 |
| **M5 — Feedback Loop** | Root-cause attribution — traces agent failures back to context bugs. | Phase 2 |
| **M6 — Governance Console** | Executive dashboard — policy enforcement, audit trail, regulation readiness. | Phase 3 |

## Project Structure

```
crates/
  domain/            Shared domain primitives (entities, value objects, events, ports)
  registry/          M1 — Context Registry bounded context
  pipeline/          M2 — Context Pipeline bounded context
  mcp-server/        MCP protocol layer (JSON-RPC 2.0 over stdio)
  cli/               CLI application (contextops binary)
  common/            Shared infrastructure adapters (in-memory implementations)
```

Each bounded context follows hexagonal architecture:

```
domain/
  entities/          Aggregate roots and entities (immutable)
  value_objects/     Immutable value types (ContentHash, ContextTier, VAID)
  events/            Domain events (collected, not fired inline)
  services/          Domain services (pure business logic)
  ports/             Interface definitions (trait-based ports)

application/
  commands/          Write use cases (one class per use case)
  queries/           Read use cases
  dtos/              Data transfer objects

infrastructure/
  repositories/      Port implementations (adapters)
  executors/         Pipeline stage executors
  container.rs       Dependency injection / composition root
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
contextops register \
  -n "billing-rules" \
  -s "org/finance" \
  -t team \
  -f claude-md \
  -o "finance-team" \
  --file ./billing-rules.md \
  -a "alice" \
  -m "initial billing context"

# List artifacts
contextops list
contextops list --tier organisation
contextops list --namespace "org/finance"

# Get artifact details
contextops get <artifact-uuid>

# Create a new version
contextops version \
  --artifact-id <uuid> \
  --file ./updated-rules.md \
  -a "bob" \
  -m "updated payment terms to net 45"

# Search across context corpus
contextops search "billing"

# Run the context pipeline
contextops pipeline run <artifact-uuid>

# Check pipeline run status
contextops pipeline status <run-uuid>

# List recent pipeline runs
contextops pipeline runs --limit 20

# Resolve inheritance chain for a namespace
contextops resolve "org/finance/billing"

# Start MCP server
contextops serve
```

## MCP Server

The MCP server exposes ContextOps™ capabilities for AI agents via the Model Context Protocol:

### Tools (write operations)

| Tool | Description |
|------|-------------|
| `register_artifact` | Register a new context artifact with initial version |
| `create_version` | Create a new version of an existing artifact |
| `deprecate_artifact` | Mark an artifact as deprecated |
| `run_pipeline` | Execute the context pipeline against an artifact |

### Resources (read operations)

| Resource URI | Description |
|---|---|
| `contextops://artifacts` | List all artifacts |
| `contextops://artifacts/{id}` | Get artifact with version history |
| `contextops://artifacts/{id}/content` | Get raw content of latest version |
| `contextops://resolve/{namespace}` | Resolve Tier 1→2→3 inheritance chain |
| `contextops://pipeline/runs` | List recent pipeline runs |
| `contextops://pipeline/runs/{id}` | Get pipeline run detail with stage results |

### MCP Server Configuration

```json
{
  "mcpServers": {
    "contextops": {
      "command": "contextops",
      "args": ["serve"]
    }
  }
}
```

## Pipeline Stages

The standard context pipeline executes as a DAG — independent stages run concurrently:

```
validate ──┬── blast-radius ────┐
           ├── regression-test ──┼── promote-staging ── promote-production
           └── security-scan ───┘
```

| Stage | Quality Gate |
|-------|-------------|
| **Validate** | Zero schema violations; zero hierarchy conflicts |
| **Blast Radius** | Change set reviewed and approved by context owner |
| **Regression Test** | Zero regression failures; drift score within threshold |
| **Security Scan** | Zero critical/high findings; no secrets in plaintext |
| **Promote: Staging** | Integration test pass rate > 99% |
| **Promote: Production** | Drift score stable; no alert threshold breaches |

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Core Services | **Rust** | Registry, pipeline, MCP server, CLI |
| ML Components | **Python** | Drift detection, security scanning (Phase 2) |
| Console UI | **React / TypeScript** | Governance dashboard (Phase 3) |
| Storage | **Firestore + GCS** | Context artifacts, golden datasets (production) |
| Events | **Pub/Sub + BigQuery** | Immutable event log, drift metrics |
| Compute | **Cloud Run** | GCP-native deployment |

## Key Concepts

| Term | Definition |
|------|-----------|
| **Context Debt** | Accumulated divergence between the context agents operate on and the current ground truth |
| **Context Drift** | Measurable divergence of agent behaviour from intended behaviour due to stale context |
| **Context Bug** | A failure where correct model + correct prompt produces wrong output due to bad context |
| **VAID** | Verifiable Agent Identity Document — binds an agent identity to its versioned context snapshot |
| **Blast Radius** | The set of agents and workflows affected by a proposed context change |
| **Inheritance Chain** | Ordered resolution of context from Tier 1 through Tier 3 |

## Roadmap

**Phase 1 — Foundation (Q3–Q4 2026)** ✅
- M1 Context Registry GA
- M2 Context Pipeline GA
- OSS release of core + CLI
- Claude Code integration via MCP

**Phase 2 — Intelligence (Q1–Q2 2027)**
- M3 Context Drift Detector GA
- M4 Context Security Gateway GA
- M5 Context Feedback Loop GA
- Harness Marketplace listing
- MLflow + Vertex AI integration

**Phase 3 — Ecosystem (Q3 2027+)**
- M6 Governance Console GA
- CNCF / Open Governance Foundation submission
- LangChain, CrewAI, AutoGen integrations
- ContextOps™ Specification v1.0

---

**ContextOps™ is a Smeyatsky Labs intellectual property asset.**

© 2026 Smeyatsky Labs. All rights reserved.
