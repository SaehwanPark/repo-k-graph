# SQLite Schema Reference

`rkg` stores repository facts in `./.rkg/rkg.db`. `rkg init` and
`rkg db reset` bootstrap the schema. `rkg index` populates repository facts,
language facts, relationship edges, documentation, test records, workspace
metadata, and derived search indexes.

Tables that own file-scoped facts use foreign keys with `ON DELETE CASCADE` so
reindexing or deleting a file removes stale child rows deterministically.

## Lifecycle Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `repositories` | `id`, repository root/path identity | `rkg init`, `rkg index` repository upsert |
| `files` | `id`, `repository_id`, `path`, `language`, `content_hash`, `line_count` | `rkg index`, `rkg files` reads it |
| `index_runs` | `id`, repository/run timestamps, status, indexed counts | `rkg index` |

## Code Graph Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `symbols` | `id`, `file_id`, `name`, `qualified_name`, `kind`, source range | `rkg index`; queried by `rkg symbols`, `rkg find`, `rkg show`, `rkg search` |
| `edges` | `id`, source/target names or ids, edge kind, confidence, source range | `rkg index`; queried by imports, callers, callees, types, decorators, tests, impact, context, pipeline, concurrency, topology |
| `docs` | `id`, `file_id`, title/qualified target, text, source range | `rkg index`; queried by `rkg docs`, `rkg doc-search`, `rkg search` |
| `tests` | `id`, `file_id`, name, qualified name, kind, parametrization flag, source range | `rkg index`; queried by `rkg tests`, `rkg test-deps`, `rkg fixtures` |

## Git Intelligence Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `git_commits` | `id`, `repository_id`, commit hash, author, authored date, message | `rkg index` Git history pass; queried by `rkg git` and `rkg cochange` |
| `git_commit_files` | `id`, `commit_id`, file path | `rkg index` Git diff parsing; queried by `rkg git` and file co-change analysis |
| `git_commit_symbols` | `id`, `commit_id`, symbol qualified name | `rkg index` Git diff-to-symbol mapping; queried by symbol co-change analysis |

## Framework And Domain Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `routes` | `id`, `file_id`, method, path, handler, response model | `rkg index`; queried by `rkg routes` |
| `pydantic_models` | `id`, `file_id`, model name, qualified name, source range | Python indexing; queried by `rkg model` |
| `pydantic_fields` | `id`, `model_id`, field name, annotation, default/required state | Python indexing; queried by `rkg model` |
| `pydantic_validators` | `id`, `model_id`, validator name, target fields | Python indexing; queried by `rkg model` |
| `android_components` | `id`, `file_id`, component type, name, class name, permission, intent actions/categories, source range | Android manifest indexing; queried by `rkg android components` |
| `android_resources` | `id`, `file_id`, resource type, name, optional value, source range | Android `res/` XML indexing and manifest permissions; queried by `rkg android resources` |
| `generated_symbols` | `symbol_id`, provenance | Kotlin KSP/KAPT and code generation indexing; queried by `rkg show` |

Android resource and component class references are also represented as virtual
symbols in `symbols` and linked through `edges` so Kotlin `R.*` usage resolves
post-indexing without a separate reference table.

## Workspace Dependency Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `cargo_packages` | `id`, package name, version, manifest path | Rust Cargo workspace indexing; queried by `rkg workspace` and `rkg deps` |
| `cargo_dependencies` | `id`, package id/name, dependency name, version/source | Rust Cargo workspace indexing; queried by `rkg deps` |
| `fsharp_projects` | `id`, project name/path, target framework, solution membership | F# workspace indexing; queried by `rkg workspace` and `rkg deps` |
| `fsharp_dependencies` | `id`, project id/name, dependency name, kind/version/path | F# workspace indexing; queried by `rkg deps` |
| `kotlin_projects` | `id`, `repository_id`, `name`, `project_path`, `target_framework`, `is_solution_member`, `generated_source_dirs` | Kotlin Gradle/Maven indexing; queried by `rkg workspace` and `rkg deps` |
| `kotlin_dependencies` | `id`, project id/name, dependency notation, kind/version | Kotlin Gradle/Maven indexing; queried by `rkg deps` |
| `swift_projects` | `id`, package/target name, manifest path | SwiftPM indexing; queried by `rkg workspace` and `rkg deps` |
| `swift_dependencies` | `id`, project id/name, dependency name, URL/path/version | SwiftPM indexing; queried by `rkg deps` |

## Concurrency Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `concurrency_spawns` | `id`, `file_id`, source symbol, spawn kind, target name, source range | Rust, Kotlin, and Swift indexing; queried by `rkg concurrency`, `rkg topology`, and `rkg impact` |
| `concurrency_channels` | `id`, `file_id`, source symbol, channel kind, sender name, receiver name, source range | Rust and Kotlin indexing; queried by `rkg concurrency`, `rkg topology`, and `rkg impact` |
| `concurrency_selects` | `id`, `file_id`, source symbol, source range | Rust and Kotlin indexing; queried by `rkg concurrency`, `rkg topology`, and `rkg impact` |

## Rust Safety Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `rust_unsafe_blocks` | `id`, `file_id`, source symbol, source range | Rust indexing; queried by `rkg safety` |
| `rust_unsafe_functions` | `id`, `file_id`, qualified name, source range | Rust indexing; queried by `rkg safety` |
| `rust_ffi_bindings` | `id`, `file_id`, source symbol, foreign item, ABI, source range | Rust indexing; queried by `rkg safety` |

## Coverage Tables

| Table | Key columns | Populated by |
| --- | --- | --- |
| `symbol_coverage` | `id`, `file_id`, `symbol_id`, report path, test suite, covered/valid lines and branches, uncovered spans | LCOV/Cobertura XML coverage parsing; queried by `rkg coverage` and `rkg import-coverage` |

## Search Tables

`docs_fts` and `symbols_fts` are SQLite FTS5 virtual tables synchronized by
database triggers. They are not counted in the core table list, but they back
`rkg search` and `rkg doc-search` with BM25 ranking. If FTS5 has no match,
documentation search falls back to deterministic `LIKE` matching.

## Cascade Behavior

- Deleting a row from `files` cascades to file-owned symbols, docs, tests,
  routes, Android components/resources, concurrency records, safety records,
  symbol coverage, and other file-scoped facts.
- Deleting Git commits cascades to associated `git_commit_files` and
  `git_commit_symbols` rows.
- Workspace dependency rows are refreshed by repository-level indexing helpers
  before new dependency facts are inserted.
- Edge resolution updates unresolved relationships after changed files are
  indexed; stale file-scoped edges are removed during file reindexing.
