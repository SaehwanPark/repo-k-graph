# CHANGELOG

## Unreleased

### Added
- Added repeatable public repository curation under `deployment/`, including a
  release spec, local-only generation/validation script, and public README
  replacement for producing the clean `SaehwanPark/repo-k-graph` release repo
  from the private development workspace.

## 1.0.12 - 2026-06-08

### Added
- Added Phase 17 Evaluation and benchmarking:
  - Configured 18 reproducible, network-isolated benchmark tasks in `fixtures/benchmarks/tasks.json` across Python, Rust, F#, Mojo, Kotlin, and Swift (3 tasks per language family).
  - Built a static grep-based baseline simulator matching target symbol word boundaries inside candidate codebases.
  - Implemented the `rkg bench` CLI subcommand executing full repo copying, temporary git workspace bootstrapping, isolated indexing, context packing, and scoring.
  - Designed scoring metrics computing file and symbol precision, recall, F1, latency, token reduction, and task success.
  - Added `scripts/run_benchmarks.sh` for convenient script-driven execution.
  - Covered the implementation with dedicated E2E integration tests and scoring unit tests.

### Changed
- Updated workspace package version to `1.0.12` according to the versioning policy.
- Updated `SPEC.md`, `CHANGELOG.md`, and `ARCHITECTURE.md` to document the completed evaluation and benchmarking features.

## 1.0.11 - 2026-06-08

### Added
- Added Phase 16.4 Swift UI and macro depth:
  - Extracted SwiftUI property wrapper attributes (`@State`, `@Binding`, `@ObservedObject`, `@EnvironmentObject`) and UIKit attributes (`@IBOutlet`, `@IBAction`) into SQLite as `ModifiedWith` edges.
  - Extracted nested view compositions, `NavigationLink`, and freestanding `#Preview` macros inside SwiftUI `View` bodies as `Calls` edges.
  - Extracted UIKit/AppKit storyboard/nib string references from instantiation calls (such as `UIStoryboard(name: "Main")`, `UINib(nibName: "CustomCell")`, and `instantiateViewController(withIdentifier: "DetailVC")`) mapping to virtual symbols `storyboard::Main`, `nib::CustomCell`, and `viewcontroller::DetailVC`.
  - Added CLI queries support and comprehensive unit/integration test coverage.
- Added checked-in Android fixture repository at `fixtures/sample-repos/kotlin-android-basic/`.

### Changed
- Updated workspace package version to `1.0.11` according to the versioning policy.
- Updated `SPEC.md`, `CHANGELOG.md`, and `ARCHITECTURE.md` to document the completed Swift UI and macro depth features.
- Updated `docs/schema-reference.md`, `docs/user-manual.md`, and `docs/dev-roadmap.md` to document Android linkage tables and CLI commands.

## 1.0.10 - 2026-06-07

### Added
- Added Phase 15.6 Kotlin Android component and resource linkage:
  - Custom XML resource parsers in `rkg-indexer::android_parser` extracting components (activities, services, content providers, broadcast receivers, application declarations, permissions, intent filters) from `AndroidManifest.xml`, layouts (implicit layout resources, IDs, and references), navigation graphs (destination IDs and target class/layout links), and values (strings, colors, dimens).
  - Extended language detection in `rkg-indexer` mapping `.xml` files to `"xml"` language.
  - Extended tree-sitter AST relationships traversal in `rkg-lang-kotlin` to extract resource usage patterns (`R.layout.*`, `R.id.*`, `R.string.*`, `R.drawable.*`, `R.color.*`, `R.dimen.*`) as property references.
  - Ingestion pipeline support in `rkg-cli` parsing XML files and persisting components in `android_components` and resources in `android_resources` tables.
  - Represented resources and manifest component classes as virtual symbols in the `symbols` table to automatically resolve code references post-indexing.
  - Added CLI `rkg android components` and `rkg android resources` subcommands formatting and outputting beautifully aligned tables.
  - Full E2E CLI integration test suite (`crates/rkg-cli/tests/android_linkage.rs`) validating parser accuracy, database persistence, and CLI command outputs.

### Changed
- Updated workspace package version to `1.0.10` according to the versioning policy.
- Updated `SPEC.md` and `ARCHITECTURE.md` to document the completed Kotlin Android component and resource linkage features.


## 1.0.9 - 2026-06-07

### Added
- Added Phase 15.5b Kotlin generated-code provenance and deeper Flow topology:
  - Custom generated source directories parsing from Gradle configurations, storing in the `kotlin_projects` table.
  - Bootstrapped `generated_symbols` table to track generated symbols and record deterministic `ksp`, `kapt`, or `generated` provenance.
  - Implemented automatic resolution of generated symbols back to annotated source classes (supporting Serializers, Room DAOs, Dagger components, and Moshi adapters).
  - Extended Flow topology parsing to support multi-source Flow operators (`combine`, `zip`) and richer collection/spawn methods (`collectLatest`, `collectIndexed`, `launchIn`).
  - Added CLI `rkg show` visualization of symbol generated provenance.
  - Added comprehensive parser unit test coverage and E2E integration test suite (`crates/rkg-cli/tests/kotlin_provenance_flow.rs`).

### Changed
- Updated workspace package version to `1.0.9` according to the versioning policy.
- Updated `SPEC.md` and `ARCHITECTURE.md` to document the completed Kotlin provenance and deeper Flow topology features.


## 1.0.8 - 2026-06-07

### Added
- Added Phase 11.7 Python functional pipeline semantics:
  - Extracted currying, partial applications, and placeholder tracking in Python parser.
  - Extended database edges schema with `ordering` and `placeholders` columns.
  - Added helper `insert_edge_with_pipeline_metadata`.
  - Resolved AST parent matching and database UNIQUE constraint issues in functional pipelines.
  - Added E2E CLI integration tests in `crates/rkg-cli/tests/pipeline_commands.rs`.

### Changed
- Updated workspace package version to `1.0.8` according to the versioning policy.


## 1.0.7 - 2026-06-06

### Added
- Added Phase 11.6 Python tensor shape and dataframe lineage depth:
  - Extracted tensor shape hints from parameter/variable annotations (e.g. `torch.Tensor[batch, 3, 224, 224]`), structured comments (`# shape: ...`), and shape unpack assignments (`batch, channels, height, width = y.shape`).
  - Extracted multi-step dataframe column lineages (e.g. `select`, `with_columns`, `agg`, `rename`, `alias`, direct assignments) represented as `col::<derived> <- <op>(col::<source>)` type references.
  - Enabled SQLite-backed type referencer query lookups matching `shape::` and `col::` targets via custom substring matching without schema changes.
  - Added CLI integration and parser unit tests validating all flows.
- Added Phase 5.4 Coverage Integration:
  - Added zero-dependency LCOV and Cobertura XML coverage report parsers (`rkg-indexer::coverage_parser`).
  - Implemented SQL database schema updates, triggers, and APIs in `rkg-db` for `symbol_coverage` persistence.
  - Implemented a unified `get_coverage_profile` query in `rkg-query` calculating statement and branch rates, uncovering line spans, and listing test breakdowns.
  - Added CLI commands `rkg import-coverage <path> [--test-suite <name>]` and `rkg coverage <target>`.
  - Added focused unit tests and a full E2E CLI integration test suite (`crates/rkg-cli/tests/coverage_commands.rs`).
- Added Phase 18.3 distribution completion documentation:
  - Added `docs/release-checklist.md` covering GitHub Release assets, shell completions, Homebrew checksum/formula update flow, and Cargo packaging validation.
  - Added `docs/schema-reference.md` documenting core SQLite tables, key columns, cascade behavior, and command population surfaces.
  - Added `docs/language-adapter-guide.md` documenting the bounded workflow for adding future `rkg-lang-*` crates.
  - Added shared workspace package description and README metadata inheritance so Cargo packaging validation includes publishable metadata without warnings.
  - Added user-manual links and regression coverage for the new distribution reference docs.
- Added Phase 9.3b agent client configuration examples:
  - Documented deterministic local stdio MCP configuration examples for Codex, Claude Code, ForgeCode, and Antigravity-style clients in `docs/user-manual.md`.
  - Added regression coverage ensuring every documented client points at `rkg mcp serve` without requiring network access, hosted account state, or GUI automation.
- Added Phase 9.3a MCP stdio transcript smoke coverage:
  - Added checked-in JSONL transcript fixtures for Codex-style, Claude Code-style, ForgeCode-style, and AntiGravity-style local stdio client flows.
  - Added process-level `rkg mcp serve` integration coverage validating initialize, ping, tool listing, successful tool calls, invalid tool-call error handling, notification behavior, and stdout/stderr separation.
- Added Phase 15.5a bounded Kotlin Flow topology support across `rkg-lang-kotlin` and `rkg-cli`:
  - Indexed direct Kotlin Flow builders (`flow`, `channelFlow`, `callbackFlow`) and one-upstream Flow transformations/collectors using the existing generic concurrency tables and `SendsTo` edges.
  - Preserved Kotlin Flow producer-to-transformer and producer-to-collector propagation through existing `rkg concurrency`, `rkg topology`, and `rkg impact` query surfaces without introducing new schema.
  - Extended Kotlin relationship extraction to retain helper calls inside supported Flow pipelines so `rkg callers` and `rkg callees` stay useful across `map`/`catch`-style chains.
  - Added parser unit coverage in `rkg-lang-kotlin` and end-to-end CLI integration coverage in `crates/rkg-cli/tests/kotlin_flow_topology.rs`.

### Changed
- Updated `SPEC.md` completion classification so completed work stays historical, `Present` contains only the active immediate target, and later queued work has concrete remaining-work guidance.
- Updated workspace package version to `1.0.7` according to the project versioning policy.
- Synchronized durable docs with current `1.0.7` release metadata, the unreleased Kotlin Flow-topology thin slice, completed MCP stdio transcript/configuration coverage, and completed Phase 18.3 distribution references.


## 1.0.6 - 2026-06-01

### Added
- Added Phase 18.1 developer distribution infrastructure:
  - Added cross-platform GitHub Actions release workflow (`.github/workflows/release.yml`) triggered on version tags (`v*.*.*`) building `rkg` binaries for Linux x86_64, macOS aarch64, and Windows x86_64 and attaching them to GitHub Releases.
  - Added `rkg-completions` standalone Clap helper binary in `crates/rkg-cli` generating shell completion scripts (bash, zsh, fish) for the `rkg` CLI.
  - Added `clap_complete` dependency to `rkg-cli` for completion script generation.
  - Extracted `Cli` and sub-command enums into `crates/rkg-cli/src/cli_def.rs` and re-exported from a new `crates/rkg-cli/src/lib.rs` so both `rkg` and `rkg-completions` binaries share a single definition.
- Added Phase 18.2 documentation refinements:
  - Added generated `## Command Reference (Generated)` section to `docs/user-manual.md` covering every `rkg` subcommand with its flags and descriptions.
  - Added `scripts/gen_command_ref.sh` script for regenerating the command reference from `rkg --help` output.
- Added SQLite FTS5 search backend for ranked symbol and documentation retrieval:
  - Bootstrapped two FTS5 virtual content tables (`docs_fts`, `symbols_fts`) in `rkg-db` with automatic `INSERT`/`DELETE` triggers to keep them in sync with the base tables.
  - Implemented `search_docs_fts` and `search_symbols_fts` APIs in `rkg-db` using the `MATCH` operator and BM25 relevance ranking (`rank` column ordering).
  - Added a `sanitize_query_for_like` helper that strips FTS5-specific syntax tokens so fallback LIKE queries never receive malformed input.
  - Exposed FTS5 search APIs and the `SymbolSearchRow` type alias in the `rkg-query` layer.
  - Implemented the new `rkg search <query>` CLI command producing unified, BM25-ranked results across both symbols and docs with clean excerpt previews.
  - Upgraded `rkg doc-search <query>` to use FTS5 under the hood, with transparent fallback to `LIKE` matching when FTS5 returns no results.
  - Added `clean_docstring_for_excerpt` helper in `rkg-cli` to strip raw comment markers (`///`, `--`, `#`) from rendered previews.
  - Added a comprehensive E2E integration test suite in `crates/rkg-cli/tests/search_commands.rs` validating FTS indexing, BM25 ranking, graceful LIKE fallback, and trigger-based cleanup on file deletion.
- Added Kotlin coroutine and channel topology support across `rkg-lang-kotlin` and `rkg-cli`:
  - Implemented tree-sitter-backed Kotlin coroutine extraction for statically obvious `launch` and `async` spawn builders.
  - Implemented static channel declaration plus send/receive pairing heuristics, along with select-block detection for Kotlin receive builders.
  - Wired Kotlin concurrency indexing into the existing generic concurrency tables and edge kinds so `rkg concurrency`, `rkg topology`, and `rkg impact` work on Kotlin coroutine/channel flows.
  - Added focused parser unit coverage in `rkg-lang-kotlin` and end-to-end CLI coverage in `crates/rkg-cli/tests/kotlin_concurrency.rs`.
- Added Phase 16.2 Swift Relationships support across `rkg-lang-swift` and `rkg-cli`:
  - Implemented robust recursive tree-sitter AST parsers in `rkg-lang-swift` to extract Swift imports (`import Foundation`, `import class UIKit.UIView`), method and function calls (including self-calls and method targets), type references (parameter, field, and return type annotations, filtering out common Swift primitive types), protocol conformances (struct/class/enum declarations), extensions, and attributes (e.g. `@objc`, `@discardableResult`).
  - Integrated Swift relationship extraction seamlessly into the CLI indexing pipeline in `rkg-cli`, persisting all edges (`Imports`, `Calls`, `ReferencesType`, `Implements`, `Extends`, `ModifiedWith`) into the local SQLite database.
  - Added comprehensive parser unit tests in `rkg-lang-swift` and full E2E CLI integration tests (`swift_relationships.rs`) validating indexing, SQLite persistence, and CLI commands (`imports`, `callers`, `callees`, `types`).
- Added Phase 16.3 Swift project and ecosystem semantics support across the workspace:
  - Statically parsed Swift Package Manager (SPM) `Package.swift` dependency graphs, mapping external packages and internal project targets/dependencies.
  - Implemented robust test case discovery for both XCTest (`XCTestCase` classes and `test...` methods) and the modern Swift Testing framework (`@Test` attribute annotations on functions).
  - Implemented Swift Concurrency Topology extraction, parsing unstructured `Task` spawns (`Task { ... }`, `Task.detached { ... }`), `AsyncStream` declarations, and asynchronous iteration loops (`for await ... in ...`).
  - Integrated Swift workspace metadata loading, test discovery, and concurrency topology parsing into the unified CLI indexing pipeline.
  - Enhanced unified CLI subcommands `rkg workspace`, `rkg deps <name>`, `rkg tests`, and `rkg concurrency` to elegantly format and render Swift-specific structures.
  - Added a comprehensive E2E integration test suite in `crates/rkg-cli/tests/swift_workspace.rs` validating index -> workspace -> deps -> tests -> concurrency workflows on Swift project hierarchies.

### Changed
- Updated workspace package version to `1.0.6` according to the project versioning policy.
- Generalized concurrency CLI wording so shared topology surfaces are no longer Rust/Tokio-specific.
- Synchronized durable docs with the actual `1.0.6` release state, Swift completion, FTS5 search availability, Phase 18 packaging, and the Kotlin coroutine/channel slice.


## 1.0.5 - 2026-06-01

### Added
- Added Phase 16.1 Swift parser and symbol extraction across the workspace:
  - Created a new dedicated workspace crate `rkg-lang-swift` utilizing the modern `tree-sitter-swift` grammar for compatibility.
  - Implemented high-fidelity recursive AST parser in `rkg-lang-swift` extracting Swift symbols (modules, structs, classes, enums, protocols/interfaces, extensions, functions, methods, properties, and type aliases) with precise source ranges and qualified dot-separated scoping.
  - Integrated Swift file discovery (`.swift`) and language mapping in `rkg-indexer`.
  - Wired Swift symbol parsing into the primary indexing pipeline of `rkg-cli` and persisted symbols in local SQLite.
  - Added comprehensive parser unit tests in `rkg-lang-swift` and full E2E CLI integration tests (`swift_symbols.rs`) validating indexing, SQLite persistence, and CLI commands (`symbols`, `find`, `show`) on Swift source files.

### Changed
- Updated workspace package version metadata to `1.0.5` according to the project language-support versioning policy (adding Swift as `1.0.5`).
- Synchronized durable docs with actual current implementation state through Phase 16.1.

## 1.0.4 - 2026-06-01

### Added
- Added Phase 15.3 Kotlin Project, Dependency & Ktor Route Extraction across the workspace:
  - Statically parsed Gradle (`build.gradle`, `build.gradle.kts`) and Maven (`pom.xml`) files using robust string scanning to extract internal module references and external package dependencies.
  - Implemented high-fidelity Ktor route AST parser in `rkg-lang-kotlin` traversing routing builders, resolving path prefix grouping hierarchies recursively, and extracting terminal HTTP methods.
  - Integrated Kotlin workspace metadata loading and per-file Ktor route extraction into the main CLI ingestion pipeline in `rkg-cli`.
  - Updated `rkg workspace`, `rkg deps <name>`, and `rkg routes` commands to seamlessly format and render Kotlin projects, dependency graphs, and Ktor routes.
  - Added comprehensive E2E integration tests in `crates/rkg-cli/tests/kotlin_workspace.rs` validating index -> workspace -> deps -> routes indexing workflows.
- Added `docs/user-manual.md` as the comprehensive user manual covering installation/build, first-run workflow, indexing, command reference, language support, MCP usage, versioning, troubleshooting, and verification.

### Changed
- Updated workspace package version metadata to `1.0.4` according to the project language-support versioning policy: Python is `1.0.0`, then Rust, F#, Mojo, and Kotlin each increment by `0.0.1`.
- Synchronized durable docs with actual current implementation state through Phase 15.3 while leaving the initial proposal unchanged.

### Fixed
- Restored workspace formatting for `rkg-lang-mojo` so `cargo fmt --all -- --check` passes.
- Closed a bounded Phase 12.7 design-conformance gap by mapping Rust model-like `Path<T>` and `web::Path<T>` route extractor payload types through the existing route dependency edge flow.

### Refactored
- Replaced local production `unwrap()` usage in CLI pipeline rendering and MCP tool-result serialization with explicit control flow and JSON-RPC error handling while preserving existing command and tool contracts.
- Refactored `rkg-cli` indexing orchestration to isolate Cargo workspace metadata loading, dependency resolution, database persistence, and per-file indexing context without changing CLI behavior.
- Replaced tuple-heavy `rkg-query` wrapper signatures with named public type aliases and removed unnecessary Clippy suppressions while preserving the existing tuple shapes.
- Added a design-conformance audit handoff documenting implemented current-phase behavior and larger future roadmap slices.
- Performed a codebase-wide code quality refactoring across `rkg-lang-python`, `rkg-db`, and `rkg-cli` to maximize idiomatic Rust patterns and apply Clean Code philosophies:
  - Dryed up duplicate Tree-sitter Parser binding and language setup in `rkg-lang-python` into a single private `parse_to_tree` helper function.
  - Replaced 10 manual `for row in rows` query result push-loops in `rkg-db` with idiomatic `rows.collect()` and guided turbofish iterators.
  - Decomposed the monolithic 450-line `run_index` function in `rkg-cli` into 8 private, single-responsibility helper functions.
  - Resolved static analysis and clippy collapsible-if warnings and structured arguments with `IndexRunSummary` to prevent long-parameter signatures.
  - Documented codebase guidelines and philosophies in `docs/best-practices.md`.

### Documentation
- Synchronized README, architecture notes, roadmap, spec, changelog, best-practices, harness docs, and workspace handoff notes with current Phase 15 Kotlin implementation state.

### Added
- Added Phase 15.1 Kotlin Language Support and Symbol Extraction across the workspace:
  - Created a new dedicated workspace crate `rkg-lang-kotlin` utilizing the modern `tree-sitter-kotlin-ng` grammar for compatibility.
  - Implemented high-fidelity recursive AST parser in `rkg-lang-kotlin` extracting Kotlin symbols (packages, classes, objects, companion objects, interfaces, functions, methods, and properties) with precise line range and qualified dot-separated scoping.
  - Integrated static AST parsing for extension functions resolving receiver type prefixes (e.g. `String.isAlpha`) automatically into qualified names.
  - Integrated Kotlin file discovery (`.kt` and `.kts`) and language mapping in `rkg-indexer`.
  - Wired Kotlin symbol parsing into the primary indexing pipeline of `rkg-cli` and persisted symbols in local SQLite.
  - Added comprehensive parser unit tests in `rkg-lang-kotlin` and full E2E CLI integration tests (`kotlin_symbols.rs`) validating indexing, SQLite persistence, and CLI commands (`symbols`, `find`, `show`) on Kotlin source files.
- Added Phase 14 Mojo Language Support across the workspace:
  - Created a new dedicated workspace crate `rkg-lang-mojo` compiling and binding directly to tree-sitter Mojo grammar via a local C grammar copy and a dynamic `build.rs` script.
  - Implemented high-fidelity recursive AST parser in `rkg-lang-mojo` extracting Mojo symbols (modules, classes, structs, traits, functions, methods), imports (`import`, `from ... import`, and relative package references), call expressions (including `self` receiver detection), type references (parameter, field, return annotations, superclasses, and supertraits), and test discovery (matching `Test` classes, `test_` functions, and parameter fixtures).
  - Integrated Mojo file discovery (`.mojo` and `.🔥`) and language adapter mapping into `rkg-indexer`.
  - Wired Mojo symbol, import, call, type reference, and test parsing into the unified main indexing pipeline of `rkg-cli` to persist Mojo facts to SQLite.
  - Resolved Mojo test dependencies (fixtures) as `ConfiguredBy` edges and similarity name matching/direct calls as `TestedBy` edges post-indexing.
  - Added comprehensive parser unit tests in `rkg-lang-mojo` and full CLI integration tests (`mojo_symbols.rs`, `mojo_relationships.rs`, and `mojo_tests.rs`) validating indexing, SQLite persistence, and CLI commands (`symbols`, `find`, `show`, `imports`, `callers`, `callees`, `types`, `tests`, `test-deps`) end-to-end.
- Added Phase 13.5 F# Tests and Ecosystem Semantics support across `rkg-lang-fsharp` and `rkg-cli`:
  - Implemented high-fidelity Tree-sitter-based F# AST parser in `rkg-lang-fsharp` to extract xUnit/NUnit test attributes on classes and member let-bindings.
  - Implemented static AST parsing for Expecto DSL test blocks (`testList`, `testCase`, `testProperty`) extracting let-bound test list definitions.
  - Implemented static AST route parsers for Giraffe pipeline routes (`GET >=> route ...`) and Saturn router builder blocks (`get ...`, `post ...`).
  - Integrated F# test and route extraction cleanly into the unified `index_single_file` ingestion pipeline in `rkg-cli` and persisted routes/tests in SQLite.
  - Resolved test fixture parameters as `ConfiguredBy` edges and similarity name matching as `TestedBy` edges post-indexing.
  - Added comprehensive unit tests in `rkg-lang-fsharp` and integration tests (`fsharp_tests.rs` and `fsharp_routes.rs`) in `rkg-cli` verifying ingestion, SQLite persistence, and CLI lookup commands (`tests`, `test-deps`, `routes`).
- Added Phase 13.4 F# Project and Dependency Graph support across `rkg-core`, `rkg-db`, `rkg-indexer`, and `rkg-cli`:
  - Defined core domain models `FSharpProject` and `FSharpDependency` in `rkg-core`.
  - Expanded SQLite schema in `rkg-db` adding `fsharp_projects` and `fsharp_dependencies` tables with CASCADE deletion constraints, performance-optimized indexes, and `DbStatus` row-count tracking fields.
  - Added persistence APIs in `rkg-db`: `insert_fsharp_project`, `insert_fsharp_dependency`, `delete_fsharp_dependencies_for_project`, `list_fsharp_projects`, and `list_fsharp_dependencies`.
  - Implemented robust pure-Rust string-scanner parsers in `rkg-indexer/fsharp_parser.rs` for `.fsproj` (NuGet `PackageReference`, `ProjectReference`, and `TargetFramework`/`TargetFrameworks`), `.sln` (project enumeration), `paket.dependencies` (global package version constraints), and `paket.references` (per-project package references).
  - Integrated F# workspace loading, Paket resolution, and solution membership detection into the `rkg index` CLI pipeline in `rkg-cli`.
  - Extended `rkg workspace` to display F# projects alongside Cargo packages with target framework and solution membership columns.
  - Extended `rkg deps <name>` to display F# NuGet packages, project references (with relative paths), and Paket-resolved packages alongside Cargo dependencies.
  - Fixed solution membership determination: pre-compute the set in `FSharpWorkspaceMetadata` (correctly joined with `repo_root`) rather than re-reading `.sln` files with relative paths during persistence.
  - Added comprehensive unit tests for all four parsers and a full end-to-end CLI integration test (`fsharp_workspace.rs`) covering `rkg index` → `rkg workspace` → `rkg deps` flows including Paket cross-file resolution.
- Added Phase 13.3 F# Language Support and Relationships Extraction across `rkg-lang-fsharp` and `rkg-cli`:
  - Implemented high-fidelity recursive AST parser in `rkg-lang-fsharp` extracting F# module opens (`open`), function/value references (handling pipeline chains `|>`, composition operators `>>`, and computation expressions `async { ... }`), and type references (handling simple type annotations and object expressions `{ new ... }`).
  - Added rusqlite F# relationships indexing helpers (`index_fsharp_file_imports`, `index_fsharp_file_calls`, `index_fsharp_file_type_references`) in `rkg-cli` and wired them to the main `index_single_file` indexing pipeline.
  - Enhanced start symbol resolver `resolve_start_symbols` in `rkg-query` to support dot-separated fully qualified names (benefiting both Python and F#).
  - Added comprehensive F# integration tests (`fsharp_relationships.rs`) validating successful extraction, indexing, resolution, and CLI querying (`imports`, `callers`, `types`, and `pipeline`) end-to-end.
- Added Phase 13.1 & 13.2 F# Language Support and Symbol Extraction across `rkg-lang-fsharp`, `rkg-indexer`, and `rkg-cli`:
  - Created a new dedicated workspace crate `rkg-lang-fsharp` leveraging the pure-Rust `tree-sitter-fsharp` grammar.
  - Implemented high-fidelity recursive AST parser in `rkg-lang-fsharp` extracting F# namespaces, nested/global modules, records, discriminated unions, classes, interfaces, active patterns, let-bound values, and members (cleaning up self-identifiers).
  - Integrated F# file discovery (`.fs`, `.fsx`, `.fsi`) and language mapping in `rkg-indexer`.
  - Seamlessly integrated F# symbol parsing into the primary indexing pipeline of `rkg-cli` and persisted symbols in the local SQLite database.
  - Added comprehensive F# integration tests (`fsharp_symbols.rs`) and parser unit tests validating successful symbol discovery, qualified dot-scoping, and query resolution via standard lookups.
- Added Phase 12.8 Rust FFI & Memory Safety support across `rkg-core`, `rkg-db`, `rkg-lang-rust`, `rkg-query`, and `rkg-cli`:
  - Defined core domain models `RustUnsafeBlock`, `RustUnsafeFunction`, and `RustFFIBinding` in `rkg-core`.
  - Expanded SQLite DB schemas and persistence APIs in `rkg-db` introducing `rust_unsafe_blocks`, `rust_unsafe_functions`, and `rust_ffi_bindings` tables with cascade deletion constraints on `file_id` and performance-optimized indexes.
  - Implemented high-fidelity Tree-sitter AST parser in `rkg-lang-rust` extracting unsafe block spans (`unsafe { ... }`), unsafe functions/traits/impl blocks, and external FFI boundary declarations (`extern "C"`, `cxx::bridge`).
  - Implemented a premium memory safety risk profiling and safe wrapper analyzer query service `get_safety_profile` inside `rkg-query` computing risk levels, safety scores, and safe wrapper percentages.
  - Wired safety metadata extraction automatically into the main indexing pipeline of `rkg-cli`.
  - Implemented a premium terminal subcommand `rkg safety <symbol_or_file>` in `rkg-cli` rendering beautifully formatted, aligned audits of safety scores, risk levels, exposed/wrapped unsafe blocks, and foreign boundary interface bindings.
  - Added comprehensive unit and integration test suites validating all parsing, persistence, query profiling, and CLI visual output flows.
- Added Phase 12.7 Rust Web Frameworks support across `rkg-lang-rust`, `rkg-db`, and `rkg-cli`:
  - Designed a Tree-sitter-based Rust web framework route parser in `rkg-lang-rust` extracting Axum `.route` call chains and Actix-web macro attributes (`#[get]`, `#[post]`, etc.) and `.to()` call chains.
  - Statically parsed handler signature parameters extracting state (`State`, `Data`, `web::Data`), request payload (`Json`, `web::Json`), and model-like path extractor (`Path`, `web::Path`) types and mapping them to the graph as `ConfiguredBy` edges.
  - Parsed generic return types (e.g. `Json<T>`) to map route `response_model` types.
  - Wired Rust route indexing into the main CLI indexing pipeline.
  - Added comprehensive unit and integration test suites validating all parsing, indexing, and route querying CLI commands.
- Added Phase 12.6 Rust Tokio Concurrency Topology static analysis and tracing support across the workspace:
  - Added `Spawns` and `SendsTo` variants to `EdgeKind` in `rkg-core` for async execution and message passing.
  - Implemented robust static async spawn tracking (`tokio::spawn`, `std::thread::spawn`) in `rkg-lang-rust`.
  - Implemented dynamic receiver/transmitter pairings tracing local variables destructuring (`let (tx, rx) = ...`) and `.send()`/`.recv()` calls.
  - Implemented `tokio::select!` block tracking.
  - Updated query engine and impact analysis BFS propagation to traverse concurrency edges.
  - Implemented a beautiful CLI subcommand `rkg topology` rendering workspace spawning and channel pathways as elegant visual graphs.
  - Added comprehensive unit and integration test coverage verifying AST extraction, DB persistence, and CLI rendering.
  - Defined core domain models `ConcurrencySpawn`, `ConcurrencyChannel`, and `ConcurrencySelect` in `rkg-core`.
  - Expanded SQLite DB schemas and persistence APIs in `rkg-db` introducing `concurrency_spawns`, `concurrency_channels`, and `concurrency_selects` tables with cascade deletion constraints on `file_id`.
  - Designed a high-fidelity Tree-sitter-based Rust AST parser in `rkg-lang-rust` extracting spawner targets (`tokio::spawn`, `std::thread::spawn`), let-binding tuple channel pair constructions, and `tokio::select!` block locations.
  - Implemented the `get_concurrency_topology` query service inside `rkg-query` to pull spawns, channel creations, and select statements in a symbol's scope.
  - Wired concurrency indexing into the main CLI indexing pipeline of `rkg-cli`.
  - Implemented a premium visual terminal subcommand `rkg concurrency <symbol>` in `rkg-cli` rendering aligned, beautifully formatted concurrent and asynchronous topologies.
  - Added comprehensive unit and integration test suites validating all parsing, persistence, query, and CLI flows.
- Added Phase 12.5 Rust Macro & Procedural Macro Expansion Intelligence support across `rkg-lang-rust` and `rkg-cli`:
  - Implemented robust `#[derive(...)]` attribute parsing in `rkg-lang-rust` to statically simulate common macro packages (Serde, Thiserror, Clap) and map them to fully qualified trait implementations.
  - Implemented a post-macro AST extraction stage in `rkg-cli` under a new `--expand` indexing option.
  - Integrated programmatic `cargo expand` execution, syntax-tree parsing, and target-file mapping to bind macro-expanded symbols and relationships back to their home files in the graph.
  - Added comprehensive unit and integration test coverage verifying simulated derive extraction and dynamic post-macro AST indexing.
- Added Phase 12.4 Rust Cargo Workspaces & Dependency Graphs support across the workspace:
  - Implemented line-by-line Cargo workspace manifest (`Cargo.toml`) and lockfile (`Cargo.lock`) parsing in `rkg-indexer` mapping package structures, workspace dependency inheritance, and exact resolved versions.
  - Expanded SQLite DB schemas and persistence APIs in `rkg-db` introducing `cargo_packages` and `cargo_dependencies` tables.
  - Implemented dynamic external import resolution during indexing to virtual external symbols (e.g. `external:tokio` in `files` table, `tokio::sync::mpsc` in `symbols` table).
  - Automatically synthesized `docs.rs` doc-linkage URLs with correct resolved lockfile package versions for virtual external symbols.
  - Implemented conditional AST traversal pruning in `rkg-lang-rust` evaluating `#[cfg(feature = "...")]` and `#[cfg(not(feature = "..."))]` attributes matching active compiler feature flags.
  - Introduced premium CLI commands `rkg workspace` and `rkg deps <package>` in `rkg-cli` rendering beautifully aligned terminal views of workspace packages and dependency topologies.
  - Added comprehensive integration tests (`cargo_workspace.rs`) verifying all features-aware AST traversal, dependency persistence, external imports target resolution, docs.rs doc-linkage mapping, and CLI rendering.
  - Implemented robust recursive Tree-sitter AST parser in `crates/rkg-lang-rust` extracting test functions (supporting both standard `#[test]` and async/custom frameworks like `#[tokio::test]`, `#[test_case]`, `#[rstest]`) and test modules (annotated with `#[cfg(test)]` or named `test`/`tests`).
  - Added support for parsing function parameters/fixtures inside Rust test definitions for advanced test dependency tracking.
  - Integrated Rust test indexing into the standard CLI indexing pipeline supporting all `.rs` files.
  - Added comprehensive end-to-end integration tests (`test_discovery.rs` and `test_linkages.rs`) verifying full test discovery, persistence, TriedBy similarity linkages, call-propagation test linkages, and CLI queries.
- Added Phase 11.5 Python ROP & Monads monadic sequence and pipeline query support across `rkg-lang-python` and `rkg-cli`:
  - Implemented robust recursive Tree-sitter AST parser in `crates/rkg-lang-python` extracting monadic pipeline method chains (`.bind()`, `.then()`, `.pipe()`) and pipelining binary operators (`>>`, `>>=`).
  - Automated translation of higher-order function references (passed to `map`, `filter`, `reduce`, `flatMap`, `apply`, `compose`, `pipe`) and partial applications (`partial`, `functools.partial`) into standard `Calls` edges in SQLite.
  - Implemented a premium terminal subcommand `rkg pipeline <symbol>` in `rkg-cli` rendering beautiful sequence trees of the pipeline's execution and tracking downstream fail-fast error blast-radius (bypassed on preceding failure) based on source-occurrence ordering.
  - Added comprehensive unit and integration test coverage verifying AST extraction, DB persistence, and CLI sequence rendering.
- Added Phase 12.2 Rust Language Relationships Support across `rkg-lang-rust`, `rkg-db`, and `rkg-cli`:
  - Implemented robust recursive Tree-sitter AST parsers in `crates/rkg-lang-rust` extracting imports (from `use` declarations), implementations (from `impl` blocks), method/function calls (from call expressions), and type references (from parameters, return types, and field types).
  - Updated the database edge resolution algorithm in `crates/rkg-db` to natively support Rust's scope resolution operator (`::`) for parent/child fallback matching.
  - Seamlessly integrated all four relationship types into the standard CLI indexing pipeline supporting `.rs` files.
  - Added comprehensive integration tests (`rust_relationships.rs`) and unit tests verifying end-to-end relationship parsing, persistence, resolution, and CLI queries (`imports`, `callers`, `types`).
- Added Phase 12.1 Rust Language Symbols Support across `rkg-lang-rust` and `rkg-cli`:
  - Implemented robust recursive Tree-sitter AST parser in `crates/rkg-lang-rust` extracting modules, functions, structs, enums, traits, impl blocks, and methods.
  - Implemented package-scoped, slash-safe qualified name resolution using scope stack.
  - Integrated Rust language symbol indexing into the standard CLI indexing pipeline supporting `.rs` files.
  - Added comprehensive end-to-end integration tests (`rust_symbols.rs`) verifying full indexing, symbols listing, find query, and show code rendering for Rust files.
- Added Phase 11.4 Python DS/ML - Jupyter Notebook, PyTorch Submodules, and Pandas/Polars Column Lineage support across `rkg-lang-python`, `rkg-indexer`, and `rkg-cli`:
  - Supported `.ipynb` file extension in indexer discovery and language detection mapping to `"jupyter"`.
  - Implemented robust Jupyter Notebook JSON deserialization handling untagged string and array source formats.
  - Integrated notebook code cell parsing running all existing AST extractions with virtual cell-based path mapping (`path/to/notebook.ipynb#cell_{idx}`).
  - Implemented robust static AST extraction of PyTorch `nn.Module` submodules as `SymbolKind::Unknown` symbols and tracked layer calls in `forward` methods.
  - Implemented static AST extraction of Pandas/Polars column reference patterns (e.g. `pl.col("name")` and string literals) from dataframe chains (`select`/`filter`/`groupby`/`agg`/`join`) represented as `col::<column>` type references.
  - Added comprehensive unit and integration test coverage verifying full ingestion, database persistence, and CLI lookup commands (`find`, `callers`, `types`).
- Added Phase 11.3 Pydantic Fields, Validators, and Dependencies across `rkg-core`, `rkg-db`, `rkg-lang-python`, and `rkg-cli`:
  - Added new core domain models `PydanticModel`, `PydanticField`, and `PydanticValidator` to `rkg-core`.
  - Expanded SQLite schema in `rkg-db` with `pydantic_models`, `pydantic_fields`, and `pydantic_validators` tables, featuring optimized indexes and CASCADE deletion rules.
  - Implemented robust database persistence and lookup helper functions in `rkg-db` for models, fields, and validators.
  - Designed a high-fidelity Tree-sitter-based Pydantic parser in `rkg-lang-python` identifying class inheritance from `BaseModel`, parsing field annotations, mapping default values and requirements (including robust `Ellipsis` or `...` matching inside `Field()`), and extracting `@field_validator` and `@model_validator` decorated blocks with targets.
  - Wired Pydantic model indexing automatically into the main indexing pipeline of `rkg-cli`.
  - Resolved model-to-model dependency linkages repository-wide as `ReferencesType` graph edges post-indexing.
  - Implemented a premium terminal subcommand `rkg model <name>` in `rkg-cli` rendering beautiful, aligned details of fields, validation rules, and structural dependencies.
  - Added comprehensive unit and integration test suites validating all parsing, persistence, dependency resolution, and query flows.
- Added Phase 11.2 Flask Route Parsing with Blueprint Linkage across `rkg-lang-python` and `rkg-cli`:
  - Implemented Flask Blueprint definition parser (`traverse_for_blueprints`) extracting blueprint variable names and their optional `url_prefix` parameters from AST assignments in `rkg-lang-python`.
  - Implemented blueprint prefix route path resolution in `traverse_routes` safely prepending blueprint prefixes with slash-safety to Flask route paths.
  - Added robust unit tests verifying Blueprint route extraction and path normalization in `rkg-lang-python`.
  - Added robust CLI integration tests verifying Flask route extraction, persistence, and listing via `rkg routes` in `rkg-cli`.
- Added Phase 11.1 FastAPI Route and Dependency Extraction across `rkg-core`, `rkg-db`, `rkg-lang-python`, and `rkg-cli`:
  - Added new core domain model `Route` in `rkg-core`.
  - Expanded SQLite schema in `rkg-db` introducing `routes` table and indexes with foreign key cascade.
  - Implemented route insertion and lookup query helper methods in `rkg-db` with collect iterators.
  - Implemented highly precise Tree-sitter-based FastAPI route decorator, response model, and implicit/explicit Depends() parameter dependency injection parser in `rkg-lang-python`.
  - Integrated routes indexing and parameter dependency registration as `ConfiguredBy` edges in `rkg-cli` indexing pipeline.
  - Added `rkg routes` CLI query subcommand in `rkg-cli` rendering aligned, premium terminal tables of endpoints.
  - Added robust end-to-end integration and unit tests covering all route and dependency features.
- Added Phase 10 Git Intelligence across `rkg-core`, `rkg-indexer`, `rkg-db`, `rkg-query`, and `rkg-cli`:
  - Added new core domain types `GitCommitInfo`, `GitFileMetadata`, `CochangeRecord`, and `CochangeAnalysis` to `rkg-core`.
  - Expanded SQLite schema in `rkg-db` introducing `git_commits`, `git_commit_files`, and `git_commit_symbols` tables with performance-optimized indexes.
  - Implemented standard spawning-based Git history log parser (`extract_git_history`) and hunk diff parser (`extract_symbol_changes_in_commit`) in `rkg-indexer` utilizing word boundaries.
  - Integrated Git log mapping directly into the main `run_index` CLI indexing pipeline post-indexing, returning soft early warnings if executed in non-Git/dummy test environments.
  - Added unified query services inside `rkg-query` computing file/symbol churn, author ratios, last modified commits, and self-joining co-change distributions.
  - Added CLI subcommands `rkg git <file>` and `rkg cochange <symbol_or_file>` in `rkg-cli` rendering beautiful, highly formatted statistics.
  - Added comprehensive integration tests in `crates/rkg-cli/tests/git_commands.rs` programmatically initializing Git repositories with multiple commits.
- Added Phase 9 Model Context Protocol (MCP) Server across `rkg-mcp` and `rkg-cli`:
  - Added line-delimited JSON-RPC 2.0 stdio MCP server in `crates/rkg-mcp` utilizing standard `serde` and `serde_json`.
  - Added a CLI command `rkg mcp serve` exposing the server over stdin/stdout.
  - Exposed 8 deterministic tools: `find_symbol`, `get_symbol`, `get_callers`, `get_callees`, `get_docs`, `get_tests`, `get_impact_analysis`, and `get_context_pack` directly wired to existing repository query engines in `rkg-query`.
  - Added a smart, highly robust substring search fallback for the `find_symbol` tool.
  - Logged all handshake warnings and runtime info exclusively to `stderr` (`eprintln!`) to prevent polluting the standard output.
  - Added a comprehensive integration test suite verifying standard initialize handshakes, tool listing, robust parameter errors, and tool executions.
- Added Phase 8 Context Packing across `rkg-query` and `rkg-cli`:
  - Added token budget estimation based on standard character heuristics.
  - Added tier-based budget trimming to optimize context selection for downstream LLM coding agents.
  - Added structured formatting for both beautiful, provenance-first Markdown and highly parsed, schema-compliant JSON outputs.
  - Added a CLI command `rkg context <symbol> [--budget <N>] [--format <fmt>]` to print packed context directly.
  - Added robust unit and integration tests verifying packing correctness and budget limitations.
- Added Phase 7 Deterministic Query Engine across `rkg-query` and `rkg-cli`:
  - Added a generic, transitive bounded-depth BFS traversal engine (`traverse_relationships`) supporting forward and backward directions over calls, imports, type references, and decorators.
  - Added a start symbol resolver (`resolve_start_symbols`) supporting exact matching on qualified names and fuzzy/simple name lookup.
  - Added a comprehensive impact analysis API (`analyze_impact`) that computes downstream dependencies, upstream blast radius, affected tests, and affected documentation blocks.
  - Added a CLI subcommand `rkg impact <symbol> --depth <N>` (with `-d` short flag, default depth of 2) rendering nested tree-like results with directional indicators.
  - Added extensive unit tests in `rkg-query` and integration tests in `crates/rkg-cli/tests/impact_commands.rs` verifying 100% linter and functional accuracy.
- Added Phase 6 Documentation Intelligence across `rkg-lang-python`, `rkg-db`, `rkg-query`, and `rkg-cli`:
  - Added lightweight Markdown heading-section parsing with line-range provenance and file ingestion support for `.md` files.
  - Added Python docstring extraction for modules, classes, methods, and functions using Tree-sitter.
  - Added SQLite database structures and access APIs (`insert_doc`, `lookup_docs_for_symbol`, `search_docs`).
  - Added a post-indexing resolution step `resolve_symbol_doc_linkages` linking symbols to their docstrings, same-name heading matches, and qualified name/import path mentions in markdown files.
  - Added new CLI commands `rkg docs <symbol>` and `rkg doc-search <query>` with elegant terminal preview truncations and formats.
  - Added robust integration tests verifying docstring parsing, markdown linkages, search, and cascade deletion behaviors.
- Added Phase 5.2 Python test-to-symbol linkage across `rkg-lang-python`, `rkg-db`, and `rkg-cli` to extract test function parameters as fixture dependencies, support `ConfiguredBy` and `TestedBy` edge persistence in SQLite, resolve unresolved test linkages post-indexing, and automatically propagate direct calls from test cases to code under test into `TestedBy` edges with high confidence.
- Added Phase 5.3 CLI test query commands in `rkg-cli`: `rkg tests <symbol>`, `rkg test-deps <test>`, and `rkg fixtures <test>` supported by a unified lookup query layer in `rkg-query` and `rkg-db`, with comprehensive end-to-end integration tests.
- Added Phase 5.1 Python test discovery (pytest discovery) across `rkg-lang-python`, `rkg-db`, and `rkg-cli` to parse pytest test classes, test functions, parametrized tests, and fixtures, expand the SQLite `tests` table schema with `kind` and `is_parametrized` attributes, support robust DB insert/lookup test APIs, and integrate test discovery directly into the CLI indexing pipeline with focused unit and integration test coverage.
- Initialized spec-driven project documents with `SPEC.md`, `ARCHITECTURE.md`, and `CHANGELOG.md`.
- Added repo-local agent harness guidance for `rkg` development workflows.
- Established the Phase 0 Rust workspace layout, CI checks, sample fixture repository, and core domain model foundation.
- Added Phase 1.1 SQLite schema bootstrap in `rkg-db`, including idempotent creation of `repositories`, `files`, `symbols`, `edges`, `docs`, `tests`, and `index_runs` tables with focused schema tests.
- Added Phase 1.2 persistence APIs in `rkg-db` for inserting files/symbols/edges, looking up files by path and symbols by name, and deleting/reindexing files with focused unit test coverage.
- Added Phase 1.3 CLI database commands in `rkg-cli`: `rkg init`, `rkg db status`, and `rkg db reset`.
- Added `rkg-db` status reporting API for schema presence and core-table row counts.
- Added focused CLI integration tests for initialization, status reporting, and reset flows.
- Added Phase 2.1 repository-ingestion foundation in `rkg-indexer`: repo-root detection, deterministic file discovery with `.gitignore` and `.rkgignore` handling, language detection, content hashing, line-count extraction, and focused ingestion tests.
- Added Phase 2.2 incremental-indexing APIs and logic across `rkg-db` and `rkg-indexer`: repository upsert/lookup, repository-scoped file snapshots, index-run start/finish metadata persistence, and deterministic `changed`/`unchanged`/`deleted` classification.
- Added Phase 2.3 CLI ingestion commands in `rkg-cli`: `rkg index`, `rkg index --force`, `rkg index --changed`, and `rkg files`, including focused CLI integration tests for incremental reruns, force mode, and deletion handling.
- Added Phase 3.1 Python parser integration across `rkg-lang-python`, `rkg-indexer`, and `rkg-cli`: Tree-sitter parsing APIs with deterministic diagnostics plus non-fatal parse-error reporting during `rkg index`, including focused parser and ingestion tests.
- Added Phase 3.2 Python symbol extraction across `rkg-lang-python`, `rkg-db`, and `rkg-cli` to extract modules, classes, and synchronous/asynchronous/nested functions, using package-scoped qualified naming conventions.
- Added Phase 3.3 CLI symbol commands in `rkg-cli`: `rkg symbols`, `rkg find`, and `rkg show`, with focused end-to-end integration tests.
- Added Phase 4.1 Python relationship extraction (import graph) across `rkg-lang-python`, `rkg-db`, `rkg-query`, and `rkg-cli` to parse absolute, from, relative, and wildcard imports, resolve targets against newly indexed repository modules/symbols via Rust-based database resolution, and query relationships via `rkg imports` and `rkg imported-by` CLI commands.
- Added Phase 4.2 Python relationship extraction (call graph) across `rkg-lang-python`, `rkg-db`, `rkg-query`, and `rkg-cli` to extract direct function calls, self-methods, constructor calls, and generic receiver method calls, persist call relationships as `Calls` edges with appropriate confidence ratings, resolve unresolved calls repository-wide post-indexing, and query callers and callees via CLI commands `rkg callers` and `rkg callees`, with comprehensive integration tests.
- Added Phase 4.3 Python relationship extraction (type references) across `rkg-lang-python`, `rkg-db`, `rkg-query`, and `rkg-cli` to parse class base types, parameter annotations, return annotations, typed assignments, and generic/typing containers while filtering out common built-ins, persist relationship facts as `ReferencesType` edges in the local SQLite store, resolve unresolved type references post-indexing, and query forward type references and backward type referencers via unified CLI query command `rkg types <name>`, with comprehensive unit and integration tests.
- Added Phase 4.4 Python relationship extraction (decorator extraction) across `rkg-lang-python`, `rkg-db`, `rkg-query`, and `rkg-cli` to parse function, class, and method decorators (handling dotted names and arguments), persist relationship facts as `ModifiedWith` edges in the SQLite database, resolve unresolved decorator references post-indexing, and query forward and backward decorator relationships via unified CLI query command `rkg decorators <name>`, with comprehensive unit and integration tests.
