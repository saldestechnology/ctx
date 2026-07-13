#!/usr/bin/env bash
# Local equivalent of the deterministic pull-request quality gates.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

python3 scripts/check-governance.py check
python3 -m unittest discover -s tests/versioning -p 'test_*.py'
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
cargo build --locked --release --no-default-features
python3 scripts/version.py check --binary target/release/ctx
python3 scripts/check-contracts.py check --binary target/release/ctx
cargo publish --locked --dry-run
cargo publish --locked --dry-run --no-default-features
