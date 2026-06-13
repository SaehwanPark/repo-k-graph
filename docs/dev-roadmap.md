---
title: "rkg Development Roadmap"
author: "Sae-Hwan Park"
date: 2026-05-27
---

# rkg Development Roadmap

## Current Status

As of release `1.0.12`, all phases (Phases 0 through 17) are fully implemented. Python support defines the `1.0.0` baseline; completed Rust, F#, Mojo, Kotlin, and Swift support incremented the language-support line to `1.0.5`; SQLite FTS5 search plus packaging/distribution refinements brought the workspace release to `1.0.6`; Phase 11.6 Python tensor shape and dataframe lineage depth brought the workspace release to `1.0.7`; Phase 11.7 Python functional pipeline semantics brought it to `1.0.8`; Phase 15.5b Kotlin generated-code provenance and deeper Flow topology brought it to `1.0.9`; Phase 15.6 Kotlin Android component and resource linkage brought it to `1.0.10`; Phase 16.4 Swift UI and macro depth brought it to `1.0.11`; and Phase 17 Evaluation and benchmarking brought the workspace release to `1.0.12`. No further roadmap phases are planned at this time.

## Phase 0 — Project Foundation

### 0.1 Repository setup

Implement:

* Rust workspace
* crate layout
* CI
* formatting/linting
* test harness
* sample repositories for fixtures

```text
crates/
  rkg-cli
  rkg-core
  rkg-db
  rkg-indexer
  rkg-query
  rkg-lang-python
  rkg-mcp
```

### 0.2 Core domain model

Implement:

* `File`
* `Symbol`
* `SymbolKind`
* `Location`
* `Edge`
* `EdgeKind`
* `DocBlock`
* `TestCase`
* `ContextPack`

---

## Phase 1 — Local Knowledge Store

### 1.1 SQLite schema

Implement tables:

* `repositories`
* `files`
* `symbols`
* `edges`
* `docs`
* `tests`
* `index_runs`

### 1.2 Basic persistence API

Implement:

* insert files
* insert symbols
* insert edges
* lookup by path
* lookup by symbol name
* delete/reindex file

### 1.3 CLI database commands

Implement:

```bash
rkg init
rkg db status
rkg db reset
```

---

## Phase 2 — Repository Ingestion

### 2.1 File discovery

Implement:

* repo root detection
* `.gitignore` support
* `.rkgignore`
* language detection
* file hashing

### 2.2 Incremental indexing

Implement:

* detect changed files
* detect deleted files
* skip unchanged files
* maintain index run metadata

### 2.3 CLI ingestion commands

Implement:

```bash
rkg index
rkg index --force
rkg index --changed
rkg files
```

---

## Phase 3 — Python Symbol Extraction

### 3.1 Python parser integration

Implement:

* Tree-sitter Python parser
* syntax tree traversal
* parse error reporting

### 3.2 Symbol extraction

Implement:

* modules
* classes
* functions
* methods
* nested functions
* async functions
* line ranges
* qualified names

Example:

```text
src/a/b.py::ClassName.method_name
```

### 3.3 CLI symbol commands

Implement:

```bash
rkg symbols
rkg find validate_patient
rkg show src/a/b.py::ClassName.method_name
```

---

## Phase 4 — Python Relationship Extraction

### 4.1 Import graph

Implement:

* `import x`
* `import x as y`
* `from x import y`
* relative imports
* unresolved imports tracking

Commands:

```bash
rkg imports src/a/b.py
rkg imported-by src/a/b.py
```

### 4.2 Call graph

Implement:

* direct function calls
* method calls where statically obvious
* constructor calls
* unresolved call references
* confidence labels

Commands:

```bash
rkg callees validate_patient
rkg callers validate_patient
```

### 4.3 Type references

Implement:

* function parameter annotations
* return annotations
* variable annotations
* class base types
* `typing` constructs
* Pydantic model references

Command:

```bash
rkg types validate_patient
```

### 4.4 Decorator extraction

Implement:

* function decorators
* class decorators
* route decorators
* pytest decorators

Command:

```bash
rkg decorators validate_patient
```

---

## Phase 5 — Python Test Intelligence

### 5.1 Pytest discovery

Implement:

* test files
* test functions
* test classes
* parametrized tests
* fixtures

### 5.2 Test-to-symbol linkage

Implement:

* name similarity
* imports used in tests
* direct calls from tests
* fixture dependencies

Commands:

```bash
rkg tests validate_patient
rkg test-deps test_validate_patient
rkg fixtures test_validate_patient
```

### 5.3 Coverage integration later

Optional:

* ingest coverage XML
* link covered lines to symbols

Command:

```bash
rkg coverage validate_patient
```

---

## Phase 6 — Documentation Intelligence

### 6.1 Markdown indexing

Implement:

* README parsing
* markdown heading hierarchy
* section extraction
* file-span provenance

### 6.2 Docstring extraction

Implement:

* module docstrings
* class docstrings
* function docstrings
* method docstrings

### 6.3 Symbol-doc linkage

Implement:

* same-name linkage
* heading mention linkage
* import path mention linkage
* docstring ownership

Commands:

```bash
rkg docs validate_patient
rkg doc-search "FHIR export"
```

---

## Phase 7 — Deterministic Query Engine

### 7.1 Symbol lookup

Implement:

* exact lookup
* fuzzy name lookup
* qualified-name lookup
* path-scoped lookup

### 7.2 Graph traversal

Implement:

* callers
* callees
* imports
* imported-by
* type references
* bounded depth traversal

### 7.3 Impact analysis

Implement:

* upstream callers
* downstream callees
* tests affected
* docs affected
* config references

Command:

```bash
rkg impact validate_patient --depth 2
```

---

## Phase 8 — Context Packing

### 8.1 Context pack model

Implement:

* provenance-first snippets
* file spans
* symbol summaries
* relationship summaries
* token budget estimation

### 8.2 Context selection

Implement:

* target symbol
* direct callers/callees
* tests
* docs
* imports
* type references

Commands:

```bash
rkg context validate_patient
rkg context validate_patient --budget 2000
rkg context validate_patient --format markdown
rkg context validate_patient --format json
```

### 8.3 Agent-focused output

Implement:

* compact Markdown
* JSON
* line-span citations
* deterministic ordering

---

## Phase 9 — MCP Server

### 9.1 MCP foundation

Implement:

* `rkg mcp serve`
* tool registry
* JSON schema outputs
* error handling

### 9.2 MCP tools

Implement:

* `find_symbol`
* `get_symbol`
* `get_callers`
* `get_callees`
* `get_docs`
* `get_tests`
* `get_impact_analysis`
* `get_context_pack`

### 9.3 Agent integration testing

Implement examples for:

* Codex
* Claude Code
* ForgeCode
* AntiGravity CLI where possible

---

## Phase 10 — Git Intelligence

### 10.1 Git metadata extraction

Implement:

* last modified commit
* author frequency
* file churn
* symbol churn if feasible

### 10.2 Co-change graph

Implement:

* files changed together
* symbols changed together
* test/code co-change relationships

Commands:

```bash
rkg git src/a/b.py
rkg cochange validate_patient
```

---

## Phase 11 — Python Framework Intelligence

### 11.1 FastAPI

Implement:

* route extraction
* request/response model linkage
* dependency injection linkage

Command:

```bash
rkg routes
```

### 11.2 Flask

Implement:

* route decorators
* blueprint linkage

### 11.3 Pydantic

Implement:

* model fields
* validators
* model dependencies

Command:

```bash
rkg model Patient
```

### 11.4 Python Data Science & ML Frameworks

Implement:

* PyTorch `nn.Module` layer and submodule static extraction
* computational graph traversal in `forward` or `call` methods
* tensor shape annotation and comment metadata propagation
* Pandas/Polars dataframe column lineage and operation tracking
* Jupyter Notebook (`.ipynb`) cell-level indexing and symbol mapping

### 11.5 Python ROP & Monads

Implement:

* monadic method chain and binding traversal (`.then()`, `.bind()`, `>>=`, `pipe`)
* functional higher-order argument references transformed into calls
* currying and partial parameter application mapping (`@curry`)
* fail-fast validation error chain propagation and blast-radius tracking (`comp-builders`, `PyMonad`)

---

## Phase 12 — Rust Language Support

### 12.1 Rust symbols

Implement:

* modules
* functions
* structs
* enums
* traits
* impl blocks
* methods

### 12.2 Rust relationships

Implement:

* imports/use statements
* trait implementations
* method calls where extractable
* type references

### 12.3 Rust tests

Implement:

* `#[test]`
* test modules
* integration tests

### 12.4 Rust Cargo Workspaces & Dependency Graphs

Implement:

* `Cargo.toml` and lockfile parsing for local/workspace crate topologies
* external crate imports resolution and doc-linkage mapping
* compiler feature flag evaluation (`features = [...]`)

### 12.5 Rust Macro & Procedural Macro Expansion Intelligence

Implement:

* static simulation/registration for common macros (`serde`, `thiserror`, `clap`)
* `cargo expand` build hook integration for post-macro AST extraction
* derived traits and implementation metadata parsing

### 12.6 Rust Asynchronous Runtime Concurrency & Tokio Topology

Implement:

* static async spawn tracking (`tokio::spawn`, dynamic threads)
* channel transmitter/receiver pairings (`tokio::sync::mpsc`, `oneshot`, `broadcast`)
* task communication topologies and concurrency select block tracing

### 12.7 Rust Web & API Frameworks

Implement:

* Axum and Actix-web REST route template parsing
* handler signature payload/context extractors mapping (`State`, `Json`, `Path`)
* end-to-end API-to-database structure tracing

### 12.8 Rust FFI & Unsafe Memory Safety

Implement:

* unsafe block (`unsafe`) and unsafe function markers indexing
* FFI bindings and raw memory interface boundaries tracking (`extern "C"`, `cxx`)
* memory safety risk profiling and safe wrapper estimation

---

## Phase 13 — F# Language Support

### 13.1 F# parser and ecosystem tooling evaluation

Implement:

* evaluate Tree-sitter F# for deterministic syntax extraction
* evaluate FSharp.Compiler.Service for compiler-backed symbols, type-aware analysis, and project-wide resolution
* document the adapter boundary between fast syntax extraction and optional compiler-service enrichment
* define fixture coverage for script files, SDK-style projects, solutions, and package-managed repositories

### 13.2 F# symbols

Implement:

* modules
* namespaces
* functions
* values
* records
* discriminated unions
* classes
* interfaces
* members
* active patterns

### 13.3 F# relationships

Implement:

* module opens
* function and value references
* type references
* object expressions
* pipeline chains
* function composition operators
* computation expressions where feasible

### 13.4 F# project and dependency graph

Implement:

* `.fsproj` and `.sln` project topology extraction
* NuGet package references
* Paket dependency metadata where present
* project reference edges and target framework metadata where feasible

### 13.5 F# tests and ecosystem semantics

Implement:

* Expecto, FsCheck, xUnit, and NUnit test discovery patterns
* Type Provider boundary tracking
* Giraffe, Saturn, and ASP.NET-style route patterns where statically extractable
* FSharpPlus and FsToolkit-style functional pipeline helpers where statically extractable

---

## Phase 14 — Mojo Language Support

Status: Implemented in release `1.0.3`.

### 14.1 Mojo parser support

Implement:

* available parser integration
* module/function/struct extraction
* imports

### 14.2 Mojo relationships

Implement:

* function calls
* type references
* Python interop references where feasible

---

## Phase 15 — Kotlin Language Support

Status: Implemented through Gradle/Maven workspace, dependency, Ktor route, coroutine/channel topology, Flow topology, KSP/KAPT generated-code provenance, and static Android manifest/resource linkage (`rkg android components`, `rkg android resources`).

### 15.1 Kotlin parser and symbol extraction

Implement:

* parser/tooling evaluation for syntax-first and compiler-backed analysis paths
* packages
* classes
* objects and companion objects
* interfaces
* functions
* properties
* extension functions and extension properties

### 15.2 Kotlin relationships

Implement:

* imports
* function and method calls
* type references
* inheritance and interface implementation
* annotations
* Java interop references where feasible

### 15.3 Kotlin project and ecosystem semantics

Implement:

* Gradle and Maven dependency graph extraction
* KSP-generated symbol boundary tracking where feasible
* coroutine, channel, and flow topology where statically extractable
* Ktor route and handler extraction
* Android component and resource linkage where feasible

---

## Phase 16 — Swift Language Support

### 16.1 Swift parser and symbol extraction

Implement:

* parser/tooling evaluation for SwiftSyntax and SourceKit-backed analysis paths
* modules
* structs
* classes
* enums
* protocols
* extensions
* functions
* properties

### 16.2 Swift relationships

Implement:

* imports
* function and method calls
* type references
* protocol conformance
* extension relationships
* attributes and macros where feasible

### 16.3 Swift project and ecosystem semantics

Implement:

* Swift Package Manager dependency graph extraction
* XCTest and Swift Testing discovery patterns
* Swift concurrency topology for tasks, async calls, actors, and async sequences where statically extractable
* SwiftUI/UIKit/AppKit structure where feasible

---

## Phase 17 — Evaluation and Benchmarking

### 17.1 Benchmark suite

Implement:

* sample tasks
* baseline agent runs
* rkg-assisted agent runs
* token measurement
* file-selection precision

### 17.2 Metrics dashboard

Track:

* indexing time
* query latency
* token reduction
* relevant file precision
* hallucinated symbol rate
* task success rate

---

## Phase 18 — Packaging and Distribution

### 18.1 Developer distribution

Implement:

* GitHub releases
* Homebrew formula
* cargo install
* shell completions

### 18.2 Documentation

Write:

* release documentation refinements beyond the current `docs/user-manual.md`
* generated command reference, if CLI help metadata becomes rich enough to support it
* deeper MCP integration examples
* language adapter guide
* schema reference

---

## Recommended MVP Cut

The first useful release should include:

```text
Phase 0
Phase 1
Phase 2
Phase 3
Phase 4.1
Phase 4.2 basic
Phase 5.1
Phase 6.1
Phase 7
Phase 8 basic
Phase 9 basic
```

MVP commands:

```bash
rkg init
rkg index
rkg symbols
rkg find
rkg show
rkg imports
rkg callers
rkg callees
rkg tests
rkg docs
rkg context
rkg mcp serve
```
