---
title: "Project Proposal: repo-k-graph (rkg)"
author: "Sae-Hwan Park"
date: 2026-05-27
description: "Deterministic Repository Knowledge Graph and Context Infrastructure for AI-Assisted Software Engineering"
---

# 1. Executive Summary

Large language model (LLM)-based coding agents increasingly assist software engineers in navigating, modifying, testing, and documenting large repositories. However, current agent workflows remain inefficient because most systems repeatedly rediscover repository structure through token-intensive prompting, heuristic semantic search, and ad hoc file exploration.

This proposal introduces **repo-k-graph (rkg)**, a deterministic repository intelligence platform designed to provide AI coding agents with structured, verifiable, and token-efficient access to repository knowledge.

rkg will construct a local repository knowledge graph from source code, documentation, tests, and version-control metadata. The system will expose deterministic query interfaces through a command-line interface (CLI) and Model Context Protocol (MCP) server, enabling agents such as Codex, ForgeCode, AntiGravity CLI, Claude Code, and other AI-assisted development systems to retrieve validated repository facts rather than relying solely on probabilistic semantic retrieval.

The proposed system will initially target Python repositories and incrementally expand to Rust, F#, and Mojo. The project will be implemented in Rust to maximize performance, portability, concurrency, and reliability.

---

# 2. Background and Significance

## 2.1 Current Limitations of AI Coding Agents

Modern coding agents face substantial limitations when interacting with large repositories:

* Excessive token consumption
* Repeated repository rediscovery
* Hallucinated APIs and symbols
* Poor cross-file reasoning
* Weak understanding of dependency relationships
* Inconsistent retrieval quality
* Lack of deterministic grounding

Existing approaches primarily rely on:

* vector embeddings,
* semantic chunk retrieval,
* heuristic search,
* editor/LSP integrations.

These methods are valuable but insufficient for precise software engineering tasks requiring verified structural understanding.

For example, an agent asked to modify a validation pipeline may need:

* upstream callers,
* downstream impacts,
* associated tests,
* related configuration,
* type dependencies,
* nearby documentation,
* historical ownership.

Today, these relationships are rediscovered repeatedly through expensive inference.

---

## 2.2 Opportunity

Software repositories already contain rich latent structure:

* call graphs,
* import graphs,
* type relationships,
* documentation linkage,
* test associations,
* git evolution patterns.

rkg aims to formalize this structure into a deterministic repository intelligence layer.

Rather than replacing semantic reasoning, rkg enables:

* deterministic retrieval first,
* semantic reasoning second.

This architecture aligns well with emerging AI-agent ecosystems and MCP-based tooling.

---

# 3. Project Objectives

The primary objective is to develop a deterministic repository intelligence system optimized for AI-assisted software engineering.

Specific aims include:

## Aim 1

Develop a scalable repository indexing engine capable of extracting structured software relationships from source repositories.

## Aim 2

Construct a deterministic query engine for symbol-level, file-level, and dependency-level repository reasoning.

## Aim 3

Provide an MCP-compatible interface enabling AI coding agents to consume repository intelligence programmatically.

## Aim 4

Evaluate reductions in:

* token consumption,
* hallucination frequency,
* repository traversal overhead,
* incorrect file modifications.

---

# 4. Innovation

rkg differs fundamentally from existing repository search systems.

| Existing Systems         | rkg                                 |
| ------------------------ | ----------------------------------- |
| Semantic retrieval       | Deterministic graph retrieval       |
| Embedding-first          | Structure-first                     |
| Chunk-oriented           | Symbol-oriented                     |
| Heuristic relevance      | Verified relationships              |
| Agent-driven exploration | Precomputed repository intelligence |
| Mostly textual           | Structural + semantic               |

Key innovations include:

* deterministic repository graph construction,
* graph-aware agent context generation,
* hybrid symbolic-semantic retrieval,
* token-budget-aware context packing,
* local-first architecture,
* language-extensible parser infrastructure,
* agent-native MCP integration.

---

# 5. Technical Approach

## 5.1 System Architecture

```text
Repository
   ↓
Language Parser Layer
   ↓
Repository Fact Extraction
   ↓
Deterministic Graph Database
   ↓
CLI + MCP Query Layer
   ↓
AI Coding Agents
```

---

## 5.2 Core Data Model

### Nodes

#### Code Symbols

* functions
* methods
* classes
* traits
* structs
* enums
* modules
* interfaces
* type aliases

#### Documentation

* README sections
* markdown headings
* ADRs
* docstrings
* comments

#### Tests

* unit tests
* fixtures
* integration tests

#### Repository Metadata

* commits
* authorship
* ownership
* configuration files

---

### Edges

| Edge Type       | Description                |
| --------------- | -------------------------- |
| imports         | module dependency          |
| calls           | function invocation        |
| defines         | symbol ownership           |
| implements      | interface implementation   |
| extends         | inheritance                |
| references_type | type usage                 |
| tested_by       | test coverage relationship |
| documented_by   | documentation linkage      |
| configured_by   | config dependency          |
| modified_with   | co-change history          |

---

## 5.3 Deterministic Query Layer

The core value proposition is deterministic repository retrieval.

Example queries:

```bash
rkg find validate_patient
rkg callers validate_patient
rkg callees validate_patient
rkg docs validate_patient
rkg tests validate_patient
rkg impact validate_patient
rkg context validate_patient --budget 2000
```

Example JSON output:

```json
{
  "symbol": "validate_patient",
  "kind": "function",
  "file": "src/validation/patient.py",
  "start_line": 42,
  "end_line": 91,
  "callers": [...],
  "callees": [...],
  "tests": [...],
  "docs": [...]
}
```

---

# 6. MCP Integration

rkg will expose repository intelligence through MCP tools.

Example MCP tools:

| Tool                | Purpose                      |
| ------------------- | ---------------------------- |
| find_symbol         | locate symbols               |
| get_callers         | upstream dependencies        |
| get_callees         | downstream dependencies      |
| get_docs            | associated documentation     |
| get_tests           | relevant tests               |
| get_context_pack    | token-aware context assembly |
| get_impact_analysis | dependency impact estimation |

This design enables:

* Codex integration,
* ForgeCode integration,
* Claude Code integration,
* custom AI workflows.

---

# 7. Language Support Strategy

## Phase 1 — Python (Primary)

Python is selected first because:

* high AI-agent usage,
* weak structural determinism,
* large ecosystem,
* complex dependency patterns,
* mixed typing practices.

Python-specific extraction targets:

* decorators,
* pytest fixtures,
* type hints,
* FastAPI/Flask routes,
* notebook support,
* dynamic imports.

---

## Phase 2 — Rust

Rust provides:

* strong static structure,
* deterministic symbol relationships,
* excellent validation environment.

---

## Phase 3 — F#

F# offers:

* advanced type systems,
* functional dependency modeling,
* module-heavy architectures.

---

## Phase 4 — Mojo

Mojo support provides:

* early ecosystem positioning,
* AI/ML developer adoption opportunities.

---

# 8. Implementation Plan

## Technology Stack

| Component       | Technology   |
| --------------- | ------------ |
| Core language   | Rust         |
| Parsing         | tree-sitter  |
| Database        | SQLite       |
| Text indexing   | Tantivy      |
| Git integration | git2         |
| CLI             | clap         |
| Serialization   | serde        |
| MCP server      | Rust MCP SDK |

---

# 9. Development Roadmap

## Milestone 1 — Core Infrastructure (Month 1–2)

Deliverables:

* repository walker,
* SQLite schema,
* indexing pipeline,
* Python parser integration,
* symbol extraction.

Commands:

```bash
rkg init
rkg index
rkg symbols
```

---

## Milestone 2 — Relationship Graph (Month 3–4)

Deliverables:

* call graph extraction,
* import graph extraction,
* type relationship indexing,
* test linkage.

Commands:

```bash
rkg callers
rkg callees
rkg tests
```

---

## Milestone 3 — Documentation Intelligence (Month 5)

Deliverables:

* markdown indexing,
* docstring association,
* README linkage,
* context assembly.

Commands:

```bash
rkg docs
rkg context
```

---

## Milestone 4 — MCP Integration (Month 6)

Deliverables:

* MCP server,
* structured JSON outputs,
* agent integration testing.

Commands:

```bash
rkg mcp serve
```

---

## Milestone 5 — Evaluation and Benchmarking (Month 7)

Evaluation metrics:

* token reduction,
* task completion accuracy,
* hallucination reduction,
* latency reduction,
* file-selection precision.

---

## Milestone 6 — Multi-Language Expansion (Month 8+)

Targets:

* Rust,
* F#,
* Mojo.

---

# 10. Evaluation Plan

The project will benchmark agent-assisted software engineering tasks with and without rkg assistance.

Metrics include:

| Metric                       | Goal |
| ---------------------------- | ---- |
| Token usage                  | ↓    |
| Incorrect file modifications | ↓    |
| Hallucinated symbols         | ↓    |
| Retrieval latency            | ↓    |
| Task success rate            | ↑    |
| Relevant file precision      | ↑    |

Example benchmark tasks:

* feature implementation,
* bug fixing,
* refactoring,
* test generation,
* API migration.

---

# 11. Risks and Mitigation

| Risk                      | Mitigation                           |
| ------------------------- | ------------------------------------ |
| Dynamic Python behavior   | partial static + heuristic inference |
| Large repository scaling  | incremental indexing                 |
| Graph explosion           | scoped traversal budgets             |
| Agent overreliance        | deterministic provenance output      |
| Multi-language complexity | adapter/plugin architecture          |

---

# 12. Expected Outcomes

Expected outcomes include:

* deterministic repository intelligence platform,
* improved AI-agent efficiency,
* reduced inference costs,
* improved software engineering accuracy,
* reusable MCP-compatible infrastructure,
* extensible multi-language ecosystem.

---

# 13. Long-Term Vision

Long-term, rkg may evolve into:

* repository operating system for AI agents,
* persistent repository memory layer,
* agent collaboration substrate,
* graph-aware autonomous development platform.

Potential future capabilities:

* incremental live indexing,
* IDE integrations,
* CI-aware graph evolution,
* semantic change prediction,
* automated architecture analysis,
* multi-repository organizational graphs.

---

# 14. Conclusion

repo-k-graph (rkg) addresses a foundational bottleneck in AI-assisted software engineering: the absence of deterministic, structured repository intelligence.

By combining:

* static analysis,
* graph-based repository modeling,
* deterministic retrieval,
* MCP-native interfaces,

rkg aims to become a core infrastructure layer for next-generation coding agents.

The proposed architecture emphasizes:

* correctness,
* token efficiency,
* interoperability,
* extensibility,
* local-first deployment.

This project has the potential to substantially improve reliability and efficiency across AI-driven software engineering workflows.
