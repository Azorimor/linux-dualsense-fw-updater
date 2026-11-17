#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo-llvm-cov >/dev/null 2>&1 && [[ -x "$HOME/.cargo/bin/cargo-llvm-cov" ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "error: cargo-llvm-cov not found in PATH. Install it with: cargo install cargo-llvm-cov" >&2
    exit 1
fi

if [[ -z "${QMAKE:-}" ]] && [[ -x "/usr/bin/qmake6" ]]; then
    export QMAKE="/usr/bin/qmake6"
fi

COVERAGE_DIR="target/coverage"
rm -rf "${COVERAGE_DIR}/html"
mkdir -p "${COVERAGE_DIR}"

cargo llvm-cov --workspace --all-targets --lcov --output-path "${COVERAGE_DIR}/lcov.info"
cargo llvm-cov report --workspace --html --output-dir "${COVERAGE_DIR}"

echo
cargo llvm-cov report --workspace --summary-only

echo
echo "Coverage reports generated:"
echo "  LCOV: ${COVERAGE_DIR}/lcov.info"
echo "  HTML: ${COVERAGE_DIR}/html/index.html"
