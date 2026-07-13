# ctx contributor instructions

<!-- governance-instructions:v1 -->

ctx is the `agentis-ctx` Rust package and installs the `ctx` binary. The root
package is not a Cargo workspace; `perf/` is an isolated, unpublished harness.
Rust 1.91 is the minimum supported version and the edition is 2021.

## Read before changing the repository

- [`governance/agent-workflow.md`](governance/agent-workflow.md) defines the
  contributor workflow.
- [`governance/versioning.md`](governance/versioning.md) is the canonical SemVer
  and compatibility policy.
- [`governance/guardrails.md`](governance/guardrails.md) distinguishes
  automated enforcement from human review.
- [`governance/releasing.md`](governance/releasing.md) is maintainer-only release
  procedure.

`docs/ is public product documentation` and the user manual. Internal policy,
maintainer procedures, CI governance, and agent rules belong in `governance/`,
never in `docs/`.

## Invariants

- Root `Cargo.toml` `[package].version` is the only manually edited product
  version. Use `python3 scripts/version.py`; do not hand-edit lockfile versions.
- Ordinary PRs do not bump versions. Release preparation must use the tooling,
  update reviewed changelog notes, and receive the required labels/review.
- Treat CLI flags/defaults, JSON, config, persisted schemas/indexes, Rust API,
  MCP/plugins, exits, platforms/packaging, and self-update as contracts.
- Add an Unreleased `CHANGELOG.md` entry for product behavior. Never hide a
  break in prose or update a contract snapshot without reviewing SemVer impact.
- Generated harness hooks/plugins/checksums/release notes are not hand-edited.
- Never tag, publish, release, push unrelated changes, weaken gates, or change
  repository settings without explicit authorization.

## Validation

```bash
python3 scripts/version.py show
python3 scripts/check-governance.py check
python3 -m unittest discover -s tests/versioning -p 'test_*.py'
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
scripts/ci.sh
```

Use `rg` for text discovery. When a current ctx binary/index is available, use
`ctx map`, `ctx query`, `ctx similar`, `ctx check`, and
`ctx score --against <base>` to complement source inspection. Report checks
that sandbox/network/platform limits prevent; do not call them passed.
