# repo-k-graph (rkg)

`repo-k-graph` (`rkg`) is a Rust project for deterministic repository knowledge
graphs and context infrastructure for AI-assisted software engineering.

Current release: `1.0.12`.

The current repository state is complete through Phase 17, Swift UI and macro
depth, evaluation/benchmarking, FTS search, packaging/distribution support, and
Kotlin Android integration:

* **Phase 0-2**: Repository foundation, local SQLite knowledge store, and
  git-ignore-aware file ingestion.
* **Phase 3-4**: AST-based Python symbol extraction and full relationship
  graphs.
* **Phase 5-6**: Python test discovery, coverage integration, and documentation
  intelligence.
* **Phase 7-8**: Graph traversal, impact analysis, and token-budgeted context
  packing.
* **Phase 9-10**: MCP stdio server support and Git intelligence.
* **Phase 11-17**: Python, Rust, F#, Mojo, Kotlin, Swift, Android, and benchmark
  intelligence.
* **Distribution**: GitHub Release workflow, shell completions, generated
  command reference, release checklist, schema reference, and language adapter
  guide.

## Project Documents

- [SPEC](SPEC.md)
- [ARCHITECTURE](ARCHITECTURE.md)
- [CHANGELOG](CHANGELOG.md)
- [User Manual](docs/user-manual.md)
- [Release Checklist](docs/release-checklist.md)
- [Schema Reference](docs/schema-reference.md)
- [Language Adapter Guide](docs/language-adapter-guide.md)
- [Initial Project Proposal](docs/project-proposal.md)
- [High-level Architecture Proposal](docs/architecture.md)
- [Development Roadmap](docs/dev-roadmap.md)

## Install

Install the published CLI with Cargo:

```sh
cargo install rkg-cli --locked
rkg --help
```

The Cargo package is `rkg-cli`. The primary installed executable is `rkg`;
Cargo also installs `rkg-completions` for shell completion generation. From a
source checkout, use `cargo run -p rkg-cli -- <command>` for the same commands.

## Workspace Layout

```text
crates/
  rkg-cli          CLI binary entry point and command executors
  rkg-core         shared domain model types
  rkg-db           SQLite persistence layer
  rkg-indexer      repository ingestion and file discovery
  rkg-query        unified query logic layer
  rkg-lang-*       language adapters
  rkg-mcp          Model Context Protocol server
fixtures/
  sample-repos/    small repositories for indexing integration tests
```

## Verification

Run these checks before finalizing code changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For release packaging validation:

```sh
cargo package --workspace --allow-dirty --list
```

## Versioning Policy

`rkg` uses a project-specific language-support release policy. Completed Python
support is the `1.0.0` baseline. Each completed additional language adapter
increments the patch version by `0.0.1`: Rust (`1.0.1`), F# (`1.0.2`), Mojo
(`1.0.3`), Kotlin (`1.0.4`), and Swift (`1.0.5`). Subsequent releases introduce
feature enhancements, search backends, test coverage integration, and deeper
language-specific modeling.

## Development Principles

- Follow functional-first design where practical.
- Keep spec-driven docs aligned with implementation state.
- Write tests for meaningful behavior, not only compile coverage.
- Use 2-space indentation.
- Keep deterministic repository facts separate from agent reasoning.
