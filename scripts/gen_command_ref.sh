#!/usr/bin/env bash
# gen_command_ref.sh — Generates a CLI command reference by running rkg --help
# and each known subcommand --help, then appends/replaces a section in
# docs/user-manual.md.
#
# Usage:
#   ./scripts/gen_command_ref.sh [--binary <path>] [--output <path>]
#
# Defaults:
#   binary  ./target/release/rkg
#   output  docs/user-manual.md
set -euo pipefail

BINARY="${RKG_BINARY:-./target/release/rkg}"
OUTPUT="${RKG_OUTPUT:-docs/user-manual.md}"
SECTION_MARKER="## Command Reference (Generated)"

# Build if needed
if [[ ! -x "${BINARY}" ]]; then
  echo "Binary not found at ${BINARY}. Building release binary..." >&2
  cargo build --release --bin rkg
fi

# Remove any existing generated section
if grep -qF "${SECTION_MARKER}" "${OUTPUT}"; then
  # Use Python for portable in-place truncation (macOS and Linux)
  python3 -c "
import sys
marker = sys.argv[1]
path = sys.argv[2]
with open(path) as f:
    content = f.read()
idx = content.find(marker)
if idx != -1:
    content = content[:idx].rstrip() + '\n'
with open(path, 'w') as f:
    f.write(content)
" "${SECTION_MARKER}" "${OUTPUT}"
fi

# Subcommand list (keep in sync with cli_def.rs)
SUBCOMMANDS=(
  init db index files symbols find show
  imports imported-by callers callees types decorators
  tests test-deps fixtures docs doc-search search
  impact context git cochange routes model pipeline
  concurrency safety workspace topology deps android mcp bench
)

{
  printf '\n'
  printf '%s\n' "${SECTION_MARKER}"
  printf '\n'
  printf '> This section is auto-generated from `rkg --help`. Do not edit manually.\n'
  printf '> To regenerate: `./scripts/gen_command_ref.sh`\n'
  printf '\n'

  # Top-level help
  printf '### `rkg`\n\n```text\n'
  "${BINARY}" --help 2>&1 || true
  printf '```\n\n'

  # Each subcommand
  for subcmd in "${SUBCOMMANDS[@]}"; do
    printf '### `rkg %s`\n\n```text\n' "${subcmd}"
    "${BINARY}" "${subcmd}" --help 2>&1 || true
    printf '```\n\n'
  done
} >> "${OUTPUT}"

echo "Command reference written to ${OUTPUT}"
