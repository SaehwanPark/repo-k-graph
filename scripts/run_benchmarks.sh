#!/usr/bin/env bash
# run_benchmarks.sh — Wrapper to compile and run the rkg benchmark suite in release mode.
# Usage:
#   ./scripts/run_benchmarks.sh [options]
# Examples:
#   ./scripts/run_benchmarks.sh
#   ./scripts/run_benchmarks.sh --json
#   ./scripts/run_benchmarks.sh --output results.md

set -euo pipefail

# Ensure we run from workspace root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}/.."

# Build the release binary
cargo build --release --bin rkg

# Run the benchmark command forwarding all arguments
./target/release/rkg bench "$@"
