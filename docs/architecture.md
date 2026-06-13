---
title: "rkg High-Level Architecture"
author: "Sae-Hwan Park"
date: 2026-05-27
---

# rkg High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                      Project Repository                     │
│  source code · tests · docs · config · notebooks · git      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Ingestion Layer                         │
│  repo walker · ignore rules · file classifier · git reader   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Language Adapter Layer                     │
│                                                             │
│  Python Adapter                                              │
│    symbols · imports · calls · decorators · pytest · types   │
│                                                             │
│  Implemented Additional Adapters                             │
│    Rust · F# · Mojo · Kotlin · Swift                         │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 Repository Fact Extraction                  │
│                                                             │
│  symbols                                                     │
│  call edges                                                  │
│  import edges                                                │
│  type references                                             │
│  test relationships                                          │
│  documentation links                                         │
│  git/co-change metadata                                      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  rkg Knowledge Store                        │
│                                                             │
│  SQLite                                                      │
│    files                                                     │
│    symbols                                                   │
│    edges                                                     │
│    docs                                                      │
│    tests                                                     │
│    git metadata                                              │
│                                                             │
│  SQLite FTS5                                                 │
│    symbol search                                             │
│    doc search                                                │
│    combined text search                                      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Query Engine                             │
│                                                             │
│  deterministic graph traversal                              │
│  symbol lookup                                               │
│  caller/callee analysis                                      │
│  test discovery                                              │
│  docs lookup                                                 │
│  impact analysis                                             │
│  token-budgeted context packing                              │
└───────────────┬───────────────────────────────┬─────────────┘
                │                               │
                ▼                               ▼
┌─────────────────────────────┐   ┌───────────────────────────┐
│          CLI Layer           │   │        MCP Server          │
│                             │   │                           │
│  rkg index                  │   │  find_symbol              │
│  rkg find                   │   │  get_callers              │
│  rkg callers                │   │  get_callees              │
│  rkg docs                   │   │  get_tests                │
│  rkg tests                  │   │  get_context_pack         │
│  rkg impact                 │   │  get_impact_analysis      │
│  rkg context                │   │                           │
└───────────────┬─────────────┘   └──────────────┬────────────┘
                │                                │
                ▼                                ▼
┌─────────────────────────────┐   ┌───────────────────────────┐
│       Human Developer        │   │       Coding Agents        │
│                             │   │                           │
│  terminal workflows          │   │  Codex                    │
│  debugging                   │   │  ForgeCode                │
│  repo exploration            │   │  AntiGravity CLI          │
│  review support              │   │  Claude Code              │
└─────────────────────────────┘   └───────────────────────────┘
```

## Core principle

rkg should separate **deterministic retrieval** from **semantic reasoning**.

```text
rkg = facts, graph, provenance, precise context
AI agent = interpretation, planning, code generation
```

## Main components

### 1. Ingestion Layer

Responsible for discovering repository contents.

```text
Inputs:
  source files
  tests
  markdown docs
  config files
  notebooks
  git history
```

Outputs normalized file records into the indexing pipeline.

---

### 2. Language Adapter Layer

Each language adapter extracts language-specific structure.

```text
Python adapter:
  modules
  classes
  functions
  methods
  imports
  calls
  decorators
  type hints
  pytest tests
  fixtures
```

Implemented additional adapters:

```text
Rust adapter
F# adapter
Mojo adapter
Kotlin adapter
Swift adapter
```

---

### 3. Fact Extraction Layer

Converts parsed code and docs into repository facts.

```text
Node facts:
  file
  symbol
  doc section
  test
  config item

Edge facts:
  imports
  calls
  defines
  references_type
  inherits
  decorated_by
  tested_by
  documented_by
```

---

### 4. Knowledge Store

Use a local embedded database first.

Current implementation:

```text
SQLite for graph facts
SQLite FTS5 for ranked text search
```

Avoid a separate graph database early. Edge tables are enough.

---

### 5. Query Engine

The most important layer.

It should support:

```text
symbol lookup
caller/callee traversal
import graph traversal
type reference lookup
test discovery
documentation lookup
impact analysis
context packing
```

This layer should be independent from CLI and MCP so both interfaces share the same logic.

---

### 6. CLI Layer

Human-facing and scriptable interface.

Example commands:

```bash
rkg index
rkg find validate_patient
rkg callers validate_patient
rkg callees validate_patient
rkg docs validate_patient
rkg tests validate_patient
rkg impact validate_patient
rkg context validate_patient --budget 2000
rkg mcp serve
```

---

### 7. MCP Server

Agent-facing interface.

Example tools:

```text
find_symbol
get_symbol
get_callers
get_callees
get_docs
get_tests
get_impact_analysis
get_context_pack
```

This is the key integration point for Codex, ForgeCode, AntiGravity CLI, Claude Code, and similar tools.

## Suggested Rust crate layout

```text
repo-k-graph/
  crates/
    rkg-cli/
    rkg-core/
    rkg-db/
    rkg-query/
    rkg-indexer/
  rkg-mcp/
  rkg-lang-python/
  rkg-lang-rust/
  rkg-lang-fsharp/
  rkg-lang-mojo/
  rkg-lang-kotlin/
  rkg-lang-swift/
```

Best boundary:

```text
rkg-core      shared domain models
rkg-indexer   ingestion and extraction orchestration
rkg-db        persistence
rkg-query     deterministic query engine
rkg-cli       command-line interface
rkg-mcp       agent interface
rkg-lang-*    language adapters
```
