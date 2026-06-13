# Language Adapter Guide

Use this guide when adding a new language adapter such as `rkg-lang-*`.

## Adapter Scope

Start with one thin slice:

- file discovery and language detection
- parser setup
- symbol extraction
- parser unit tests
- CLI indexing
- persistence and query coverage
- end-to-end integration tests
- documentation sync in `SPEC.md` and `CHANGELOG.md`

Do not start with framework extraction, workspace manifests, test discovery, or
special runtime semantics unless the symbol and relationship slices already
work.

## Crate Boundary

Create a dedicated crate named `rkg-lang-*` for language-specific parsing. Keep
domain-neutral records in `rkg-core`, database writes in `rkg-db`, file
discovery in `rkg-indexer`, and command orchestration in `rkg-cli`.

The language crate should expose deterministic parser functions that accept
source text plus a repository-relative path or module name. It should return
plain domain records or adapter-local parse records, not database connections or
CLI formatting.

## File Discovery

Wire file discovery in `rkg-indexer`:

- add language detection for the language's file extensions
- add fixtures under `fixtures/sample-repos/` when practical
- preserve deterministic ordering and ignore semantics
- add focused tests for extension mapping and discovery behavior

## Parser Implementation

Prefer a pure parser boundary with no filesystem or database access. For
Tree-sitter adapters, centralize parser setup in a private helper so tests and
extractors do not duplicate grammar initialization.

Parser unit tests should cover:

- empty or malformed source where supported
- top-level symbols
- nested symbols and qualified names
- source ranges
- representative language-specific declarations

## CLI Indexing

Add CLI indexing in the same orchestration style as existing adapters:

- read file content at the CLI boundary
- invoke parser functions in language-specific helpers
- insert files, symbols, docs, tests, and edges through `rkg-db`
- run relationship resolution after changed files are indexed
- keep command output unchanged unless the slice explicitly adds a command

## Persistence And Query Wiring

Use existing generic tables whenever they express the fact correctly:

- `symbols` for definitions
- `edges` for imports, calls, type references, decorators/attributes,
  implementations, inheritance, test linkages, concurrency relations, and
  pipeline relations
- `docs` for comments, docstrings, and markdown blocks
- `tests` for test cases and fixtures
- workspace-specific tables only when the language has project/package metadata
  that cannot fit existing dependency tables

Add new schema only when multiple query surfaces need a fact that cannot be
represented by existing tables. If schema changes are required, add database
tests for bootstrap, insertion, lookup, deletion, and `ON DELETE CASCADE`.

## Tests

Add tests in this order:

1. parser unit tests in `rkg-lang-*`
2. discovery tests in `rkg-indexer` if extensions or ignore behavior changed
3. database tests if persistence APIs or schema changed
4. CLI integration tests under `crates/rkg-cli/tests`

Integration tests should run `rkg init`, `rkg index`, and the relevant query
commands against a small fixture repository. Avoid network access and generated
runtime state.

## Documentation Sync

When a slice is complete:

- update `SPEC.md` Past/Present/Future status
- update `CHANGELOG.md` Unreleased
- update `README.md`, `ARCHITECTURE.md`, and `docs/user-manual.md` if language
  support or command behavior changed
- keep long-term proposal documents historical unless explicitly asked to revise
  product intent

Stop and report if adding the adapter requires broad crate-boundary changes,
runtime tool execution, public CLI behavior changes, or schema changes outside
the planned slice.
