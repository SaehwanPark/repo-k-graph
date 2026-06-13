# rkg User Manual

`repo-k-graph` (`rkg`) builds a deterministic local repository knowledge graph
from source code, tests, documentation, framework metadata, workspace manifests,
and Git history. It stores verified facts in SQLite and exposes them through a
CLI and a Model Context Protocol (MCP) stdio server.

Current release: `1.0.12`.

## Supported Languages

| Language | Release | Current support |
| --- | ---: | --- |
| Python | `1.0.0` | Symbols, imports, calls, type references, decorators, pytest tests and fixtures, docs/docstrings, FastAPI, Flask, Pydantic, Jupyter, PyTorch, Pandas/Polars, and functional pipelines. |
| Rust | `1.0.1` | Symbols, imports, calls, type references, tests, Cargo workspaces/dependencies, macro expansion support, Tokio concurrency topology, Axum/Actix routes, and unsafe/FFI safety profiling. |
| F# | `1.0.2` | Symbols, relationships, xUnit/NUnit/Expecto tests, `.fsproj`/`.sln`/Paket workspace metadata, Giraffe routes, and Saturn routes. |
| Mojo | `1.0.3` | Symbols, imports, calls, type references, Python interop references, and test discovery. |
| Kotlin | `1.0.4`–`1.0.10` | Symbols, imports, calls, type references, inheritance, annotations, Gradle/Maven workspace metadata, dependency graphs, Ktor routes, coroutine/channel/Flow topology, KSP/KAPT generated-code provenance, and static Android manifest/resource linkage (`rkg android components`, `rkg android resources`). |
| Swift | `1.0.5` | Symbols, imports, calls, type references, protocol conformances, extensions, XCTest and Swift Testing discovery, SwiftPM metadata, and concurrency topology. |

## Android Linkage (Kotlin)

For Android modules detected from Gradle (`com.android.application` /
`com.android.library`), `rkg index` statically parses:

- `AndroidManifest.xml` for activities, services, receivers, providers,
  application classes, permissions, and intent filters
- `res/layout`, `res/values`, `res/navigation`, and drawable XML for resource
  definitions and cross-references
- Kotlin `R.layout.*`, `R.id.*`, `R.string.*`, `R.drawable.*`, `R.color.*`, and
  `R.dimen.*` references in handler code

Query indexed Android facts:

```sh
cargo run -p rkg-cli -- android components
cargo run -p rkg-cli -- android resources
```

A checked-in fixture lives at
`fixtures/sample-repos/kotlin-android-basic/`.

Static analysis only: `rkg` does not run the Android Gradle Plugin, AAPT, or
device/emulator tooling.

## Install And Build

Prerequisites:

- Rust toolchain with the workspace's 2024 edition support.
- Git, for repository root detection and Git history indexing.
- Optional `cargo expand`, only when running `rkg index --expand` for Rust macro-expanded indexing.

### Install From crates.io

Install the published CLI with Cargo:

```sh
cargo install rkg-cli --locked
rkg --help
```

The Cargo package is `rkg-cli`. The primary installed executable is `rkg`;
Cargo also installs `rkg-completions` for shell completion generation.

### Build From Source

From a source checkout:

```sh
cargo build --workspace
cargo run -p rkg-cli -- --help
```

Commands in this manual use the source-checkout form
`cargo run -p rkg-cli -- <command>`. If you installed the CLI with Cargo,
replace that prefix with `rkg`.

Distribution maintainers should use the release and packaging checklist in
`docs/release-checklist.md`. Schema details are documented in
`docs/schema-reference.md`, and future language support should follow
`docs/language-adapter-guide.md`.

## First Run

Initialize the local knowledge store:

```sh
cargo run -p rkg-cli -- init
cargo run -p rkg-cli -- db status
```

Index the repository:

```sh
cargo run -p rkg-cli -- index
```

By default, indexing is incremental. It discovers files, compares persisted file
hashes, skips unchanged files, deletes records for removed files, extracts facts
for changed files, resolves unresolved edges, and stores run metadata.

Use these indexing modes when needed:

```sh
cargo run -p rkg-cli -- index --force
cargo run -p rkg-cli -- index --changed
cargo run -p rkg-cli -- index --expand
```

- `--force` reindexes discovered files and tracks deletions.
- `--changed` is an explicit alias for the default incremental flow.
- `--expand` enables the Rust macro expansion indexing path where supported by
  local tooling.

## Database Lifecycle

`rkg` stores its local SQLite database at `./.rkg/rkg.db`.

```sh
cargo run -p rkg-cli -- init
cargo run -p rkg-cli -- db status
cargo run -p rkg-cli -- db reset
```

- `init` creates the local database and schema.
- `db status` reports database path, existence, schema state, and table counts.
- `db reset` recreates the local database for the current repository.

## Core Exploration

List indexed files and symbols:

```sh
cargo run -p rkg-cli -- files
cargo run -p rkg-cli -- symbols
```

Find and inspect symbols:

```sh
cargo run -p rkg-cli -- find <name>
cargo run -p rkg-cli -- show <qualified_name>
```

`find` uses simple-name lookup. `show` expects the qualified name printed by
`symbols` or `find` and renders provenance plus a source snippet.

## Relationship Queries

Use these commands after indexing:

```sh
cargo run -p rkg-cli -- imports <file_path>
cargo run -p rkg-cli -- imported-by <file_path>
cargo run -p rkg-cli -- callers <symbol_name>
cargo run -p rkg-cli -- callees <symbol_name>
cargo run -p rkg-cli -- types <symbol_name>
cargo run -p rkg-cli -- decorators <symbol_name>
```

- `imports` and `imported-by` query file-level dependency direction.
- `callers` and `callees` query call graph direction.
- `types` queries type references forward and backward.
- `decorators` queries decorator or annotation relationships.

Relationship resolution is static and deterministic. Some dynamic language
features are recorded with unresolved or lower-confidence targets when no unique
static match exists.

## Tests And Fixtures

```sh
cargo run -p rkg-cli -- tests <symbol_name>
cargo run -p rkg-cli -- test-deps <test_name>
cargo run -p rkg-cli -- fixtures <test_name>
```

- `tests` shows tests linked to a symbol by direct calls or similarity.
- `test-deps` shows dependencies for a test.
- `fixtures` focuses on configured fixture-style dependencies.

Python pytest, Rust test attributes/modules, F# xUnit/NUnit/Expecto, and Mojo
test conventions are supported. Kotlin test discovery is not yet a completed
feature.

## Documentation Queries

```sh
cargo run -p rkg-cli -- docs <symbol_name>
cargo run -p rkg-cli -- doc-search "<query>"
```

`docs` returns documentation blocks linked to a symbol through docstrings,
same-name headings, or qualified-name mentions. `doc-search` uses SQLite FTS5
with BM25 ranking and falls back to `LIKE` matching when FTS returns no hits.

For combined ranked symbol and documentation search:

```sh
cargo run -p rkg-cli -- search "<query>"
```

## Impact And Context

```sh
cargo run -p rkg-cli -- impact <symbol_name> --depth 2
cargo run -p rkg-cli -- context <symbol_name> --budget 2000
cargo run -p rkg-cli -- context <symbol_name> --format json
```

- `impact` traverses upstream and downstream relationships, affected tests, and
  affected docs to show likely blast radius.
- `context` builds a deterministic context pack for downstream coding agents.
- `--budget` controls approximate token budget.
- `--format` accepts `markdown` or `json`.

## Git And Co-change Intelligence

```sh
cargo run -p rkg-cli -- git <file_path>
cargo run -p rkg-cli -- cochange <symbol_name_or_file_path>
```

`git` reports indexed history metadata such as churn and last modification.
`cochange` reports files or symbols that historically changed together.

Git indexing is best-effort. In non-Git or synthetic test directories, the
indexer reports a warning and continues with repository facts that do not depend
on Git history.

## Framework And Ecosystem Queries

Routes:

```sh
cargo run -p rkg-cli -- routes
```

Supported route extraction includes FastAPI, Flask, Rust Axum/Actix-web, F#
Giraffe/Saturn, and Kotlin Ktor.

Pydantic models:

```sh
cargo run -p rkg-cli -- model <model_name>
```

Functional pipelines:

```sh
cargo run -p rkg-cli -- pipeline <symbol_name>
```

Concurrency and topology:

```sh
cargo run -p rkg-cli -- concurrency <symbol_name>
cargo run -p rkg-cli -- topology
```

Rust safety profile:

```sh
cargo run -p rkg-cli -- safety <symbol_or_file>
```

Workspaces and dependencies:

```sh
cargo run -p rkg-cli -- workspace
cargo run -p rkg-cli -- deps <package_or_project_name>
```

`workspace` and `deps` currently cover Cargo, F#, Kotlin, and Swift workspace metadata.

## MCP Server

Start the stdio MCP server:

```sh
cargo run -p rkg-cli -- mcp serve
```

The server exposes deterministic query tools for coding agents:

- `find_symbol`
- `get_symbol`
- `get_callers`
- `get_callees`
- `get_docs`
- `get_tests`
- `get_impact_analysis`
- `get_context_pack`

The MCP server speaks JSON-RPC 2.0 over stdio. Runtime warnings and diagnostics
are written to stderr so stdout remains available for protocol messages.

Local stdio compatibility is covered by checked-in JSONL transcript fixtures in
`crates/rkg-cli/tests/fixtures/mcp-transcripts/`. They exercise initialize,
tool listing, tool calls, error responses, notifications, and stdout/stderr
separation without requiring network access or hosted agent accounts.

### Agent Client Configuration

Build the binary or use an installed `rkg`, then prefer an absolute path in
client configuration:

```sh
cargo build --release --bin rkg
realpath target/release/rkg
```

For an already installed binary, use `which rkg`. The examples below use
`/absolute/path/to/rkg` as the command placeholder. Replace it with the built or
installed binary path for the local machine.

### Codex

This registers `rkg mcp serve` as a local stdio MCP server.

CLI registration:

```sh
codex mcp add rkg -- /absolute/path/to/rkg mcp serve
```

Equivalent `~/.codex/config.toml` entry:

```toml
[mcp_servers.rkg]
command = "/absolute/path/to/rkg"
args = ["mcp", "serve"]
```

### Claude Code

This registers `rkg mcp serve` as a local stdio MCP server.

CLI registration:

```sh
claude mcp add --transport stdio rkg -- /absolute/path/to/rkg mcp serve
```

Project-local `.mcp.json` entry:

```json
{
  "mcpServers": {
    "rkg": {
      "command": "/absolute/path/to/rkg",
      "args": ["mcp", "serve"]
    }
  }
}
```

### ForgeCode

This registers `rkg mcp serve` as a local stdio MCP server.

CLI import:

```sh
forge mcp import '{"mcpServers":{"rkg":{"command":"/absolute/path/to/rkg","args":["mcp","serve"]}}}'
```

Project-local `.mcp.json` entry:

```json
{
  "mcpServers": {
    "rkg": {
      "command": "/absolute/path/to/rkg",
      "args": ["mcp", "serve"]
    }
  }
}
```

### Antigravity

This registers `rkg mcp serve` as a local stdio MCP server.

Profile `mcp_config.json` entry:

```json
{
  "mcpServers": {
    "rkg": {
      "command": "/absolute/path/to/rkg",
      "args": ["mcp", "serve"],
      "cwd": "/absolute/path/to/repository"
    }
  }
}
```

These examples intentionally use local stdio only. Hosted account state,
authentication, GUI setup, and remote transports are out of scope for `rkg`'s
deterministic MCP smoke coverage.

## Versioning

`rkg` uses a project-specific language-support release policy:

- Python support completion defines `1.0.0`.
- Each completed additional language adapter increments the patch version by
  `0.0.1`.
- Rust, F#, Mojo, Kotlin, and Swift bring the language-support line to `1.0.5`.
- SQLite FTS5 search and packaging/distribution refinements bring the workspace
  release to `1.0.6`.
- Python tensor shape and dataframe lineage depth bring the workspace release
  to `1.0.7`.
- Python functional pipeline semantics brings the workspace release to `1.0.8`.
- Kotlin generated-code provenance and deeper Flow topology bring the workspace
  release to `1.0.9`.
- Kotlin Android component and resource linkage brings the workspace release
  to `1.0.10`.
- Swift UI and macro depth brings the workspace release to `1.0.11`.
- Evaluation and benchmarking brings the current workspace release to `1.0.12`.

This policy is intentionally project-specific. For general release vocabulary,
the project follows the spirit of [Semantic Versioning](https://semver.org/) and
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), while documentation is
organized around practical user tasks consistent with the
[Diataxis](https://diataxis.fr/) documentation model.

## Troubleshooting

- Run `rkg db status`, or `cargo run -p rkg-cli -- db status` from a source
  checkout, first when commands return no data.
- Run `rkg index --force`, or `cargo run -p rkg-cli -- index --force` from a
  source checkout, after large file moves or branch changes.
- Use exact qualified names from `rkg symbols` or `rkg find` when `show`,
  `impact`, or `context` cannot resolve a target.
- Install `cargo expand` before using `rkg index --expand`.
- If Git metadata is missing, confirm the repository has commits and that Git is
  available on `PATH`.

## Verification For Contributors

Before finalizing code or behavior changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For documentation and release changes, also verify:

```sh
cargo metadata --no-deps --format-version 1
cargo run -p rkg-cli -- --help
```

Then check that durable docs agree on the current version, implemented language
support, and available command surface.

## Command Reference (Generated)

> This section is auto-generated from `rkg --help`. Do not edit manually.
> To regenerate: `./scripts/gen_command_ref.sh`

### `rkg`

```text
repo-k-graph CLI

Usage: rkg <COMMAND>

Commands:
  init             Initialize the local knowledge store at `./.rkg/rkg.db`
  index            Index (or reindex) repository files into the knowledge store
  files            List all indexed files in the repository
  symbols          List all indexed symbols in the repository
  find             Find symbols by simple name
  show             Show definition snippet and metadata for a fully qualified symbol
  imports          List import edges for a file path
  imported-by      List files that import the given file path
  callees          List symbols called by the given symbol
  callers          List symbols that call the given symbol
  types            List type references for the given symbol
  decorators       List decorators applied to or by the given symbol
  tests            List tests linked to the given symbol
  test-deps        List symbols that the given test depends on
  fixtures         List fixture dependencies for the given test
  docs             Show documentation linked to the given symbol
  doc-search       Search documentation blocks using BM25-ranked FTS5 (falls back to LIKE)
  search           Search symbols and docs using BM25-ranked FTS5
  impact           Show transitive impact analysis for a symbol
  context          Pack context for a symbol into a token-budgeted representation
  git              Show Git metadata (author frequency, churn, last commit) for a file
  cochange         Show co-change analysis for a symbol or file
  routes           List indexed HTTP routes
  model            Show Pydantic model fields, validators, and dependencies
  pipeline         Render monadic/ROP pipeline execution sequence for a symbol
  concurrency      Show concurrency topology for a symbol
  safety           Show Rust FFI and memory-safety profile for a symbol or file
  coverage         Show coverage profile for a symbol or file
  import-coverage  Import a coverage report (LCOV or Cobertura XML)
  workspace        List workspace packages (Cargo, F#, Kotlin, Swift)
  topology         Show async spawn and channel topology for the workspace
  deps             Show dependency graph for a workspace package
  android          Show Android components and resources
  db               Database lifecycle commands
  mcp              Model Context Protocol server commands
  bench            Run evaluation and benchmarks on fixture repositories
  help             Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `rkg init`

```text
Initialize the local knowledge store at `./.rkg/rkg.db`

Usage: rkg init

Options:
  -h, --help  Print help
```

### `rkg db`

```text
Database lifecycle commands

Usage: rkg db <COMMAND>

Commands:
  status  Report database path, schema presence, and core-table row counts
  reset   Recreate the local database from scratch (destructive)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `rkg index`

```text
Index (or reindex) repository files into the knowledge store

Usage: rkg index [OPTIONS]

Options:
      --force    Force full reindex of all discovered files
      --changed  Alias for the default incremental index behaviour (changed files only)
      --expand   Run post-macro AST extraction via `cargo expand` after symbol indexing
  -h, --help     Print help
```

### `rkg files`

```text
List all indexed files in the repository

Usage: rkg files

Options:
  -h, --help  Print help
```

### `rkg symbols`

```text
List all indexed symbols in the repository

Usage: rkg symbols

Options:
  -h, --help  Print help
```

### `rkg find`

```text
Find symbols by simple name

Usage: rkg find <NAME>

Arguments:
  <NAME>  Simple (unqualified) symbol name to look up

Options:
  -h, --help  Print help
```

### `rkg show`

```text
Show definition snippet and metadata for a fully qualified symbol

Usage: rkg show <QUALIFIED_NAME>

Arguments:
  <QUALIFIED_NAME>  Fully qualified symbol name (e.g. `src/a/b.py::ClassName.method`)

Options:
  -h, --help  Print help
```

### `rkg imports`

```text
List import edges for a file path

Usage: rkg imports <PATH>

Arguments:
  <PATH>  Repository-relative file path

Options:
  -h, --help  Print help
```

### `rkg imported-by`

```text
List files that import the given file path

Usage: rkg imported-by <PATH>

Arguments:
  <PATH>  Repository-relative file path

Options:
  -h, --help  Print help
```

### `rkg callers`

```text
List symbols that call the given symbol

Usage: rkg callers <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg callees`

```text
List symbols called by the given symbol

Usage: rkg callees <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg types`

```text
List type references for the given symbol

Usage: rkg types <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg decorators`

```text
List decorators applied to or by the given symbol

Usage: rkg decorators <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg tests`

```text
List tests linked to the given symbol

Usage: rkg tests <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg test-deps`

```text
List symbols that the given test depends on

Usage: rkg test-deps <NAME>

Arguments:
  <NAME>  Simple or qualified test name

Options:
  -h, --help  Print help
```

### `rkg fixtures`

```text
List fixture dependencies for the given test

Usage: rkg fixtures <NAME>

Arguments:
  <NAME>  Simple or qualified test name

Options:
  -h, --help  Print help
```

### `rkg docs`

```text
Show documentation linked to the given symbol

Usage: rkg docs <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg doc-search`

```text
Search documentation blocks using BM25-ranked FTS5 (falls back to LIKE)

Usage: rkg doc-search <QUERY>

Arguments:
  <QUERY>  Free-text search query

Options:
  -h, --help  Print help
```

### `rkg search`

```text
Search symbols and docs using BM25-ranked FTS5

Usage: rkg search <QUERY>

Arguments:
  <QUERY>  Free-text search query

Options:
  -h, --help  Print help
```

### `rkg impact`

```text
Show transitive impact analysis for a symbol

Usage: rkg impact [OPTIONS] <SYMBOL>

Arguments:
  <SYMBOL>  Simple or qualified symbol name

Options:
  -d, --depth <DEPTH>  Maximum traversal depth [default: 2]
  -h, --help           Print help
```

### `rkg context`

```text
Pack context for a symbol into a token-budgeted representation

Usage: rkg context [OPTIONS] <SYMBOL>

Arguments:
  <SYMBOL>  Simple or qualified symbol name

Options:
  -b, --budget <BUDGET>  Maximum token budget for the output
  -f, --format <FORMAT>  Output format: `markdown` or `json` [default: markdown]
  -h, --help             Print help
```

### `rkg git`

```text
Show Git metadata (author frequency, churn, last commit) for a file

Usage: rkg git <PATH>

Arguments:
  <PATH>  Repository-relative file path

Options:
  -h, --help  Print help
```

### `rkg cochange`

```text
Show co-change analysis for a symbol or file

Usage: rkg cochange <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name, or repository-relative file path

Options:
  -h, --help  Print help
```

### `rkg routes`

```text
List indexed HTTP routes

Usage: rkg routes

Options:
  -h, --help  Print help
```

### `rkg model`

```text
Show Pydantic model fields, validators, and dependencies

Usage: rkg model <NAME>

Arguments:
  <NAME>  Model class name

Options:
  -h, --help  Print help
```

### `rkg pipeline`

```text
Render monadic/ROP pipeline execution sequence for a symbol

Usage: rkg pipeline <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg concurrency`

```text
Show concurrency topology for a symbol

Usage: rkg concurrency <NAME>

Arguments:
  <NAME>  Simple or qualified symbol name

Options:
  -h, --help  Print help
```

### `rkg safety`

```text
Show Rust FFI and memory-safety profile for a symbol or file

Usage: rkg safety <TARGET>

Arguments:
  <TARGET>  Simple or qualified symbol name, or repository-relative file path

Options:
  -h, --help  Print help
```

### `rkg workspace`

```text
List workspace packages (Cargo, F#, Kotlin, Swift)

Usage: rkg workspace

Options:
  -h, --help  Print help
```

### `rkg topology`

```text
Show async spawn and channel topology for the workspace

Usage: rkg topology

Options:
  -h, --help  Print help
```

### `rkg deps`

```text
Show dependency graph for a workspace package

Usage: rkg deps <PACKAGE>

Arguments:
  <PACKAGE>  Package name as declared in its manifest

Options:
  -h, --help  Print help
```

### `rkg android`

```text
Show Android components and resources

Usage: rkg android <COMMAND>

Commands:
  components  List Android manifest components (Activities, Services, etc.) and their details
  resources   List Android resources (layouts, strings, IDs) and their references
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `rkg mcp`

```text
Model Context Protocol server commands

Usage: rkg mcp <COMMAND>

Commands:
  serve  Start the MCP stdio server
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### `rkg bench`

```text
Run evaluation and benchmarks on fixture repositories

Usage: rkg bench [OPTIONS]

Options:
      --config <CONFIG>  Optional path to custom tasks JSON file
      --json             Print raw JSON results to stdout
  -o, --output <OUTPUT>  Save result output to the specified file path
  -h, --help             Print help
```
