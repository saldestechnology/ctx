#!/usr/bin/env bash
# Complete, side-effect-free release gate. Does not tag, publish, or commit.
set -euo pipefail

if [[ $# -ne 1 || ! "$1" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
  echo "usage: scripts/release-check.sh v<semver>" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"
tag="$1"
version="${tag#v}"

python3 scripts/check-governance.py check
python3 -m unittest discover -s tests/versioning -p 'test_*.py'
python3 scripts/version.py check --tag "$tag" --release --skip-binary
python3 scripts/version.py notes "$version" --output release-notes.md
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
cargo build --locked --release
python3 scripts/version.py check --tag "$tag" --release --binary target/release/ctx
python3 scripts/check-contracts.py check --binary target/release/ctx
cargo publish --locked --dry-run
cargo publish --locked --dry-run --no-default-features

echo "OK: $tag passed all release gates; release-notes.md is ready for review"
