# Versioning policy

ctx follows [Semantic Versioning 2.0.0](https://semver.org/). The only manually
edited project version is `[package].version` in the root `Cargo.toml`.
`Cargo.lock`, `perf/Cargo.lock`, compiled binaries, generated plugins, release
archives, and release tags are derived from or checked against it.

`perf/` is an independent, unpublished harness at `0.0.0`; it does not share
the product release number.

## Release levels

Patch releases contain backwards-compatible bug/correctness fixes,
performance improvements without intentional incompatibility, internal
refactors, documentation fixes, compatible dependency updates, and compatible
packaging fixes.

Minor releases contain backwards-compatible functionality: new commands or
flags with compatible defaults, additive fields in explicitly extensible
formats, new analysis capabilities/integrations, and deprecations that retain
existing behavior.

Major releases contain incompatible changes: command/flag removal or rename,
incompatible semantics/defaults/configuration/machine-output changes, an index
format change without migration, incompatible Rust library API changes,
exit-code changes, platform removal, an MSRV increase outside policy, or an
incompatible plugin/MCP/protocol/integration contract.

## Pre-1.0 compatibility

Until 1.0, `0.MINOR.0` may intentionally break compatibility.
`0.MINOR.PATCH` must remain compatible within that minor line. Every break
still needs maintainer acknowledgement (`breaking-change` and
`contract-review` labels), a prominent `BREAKING:` changelog entry, the
required version increase, migration guidance, and deprecation first when
reasonably possible. Pre-1.0 is not permission for accidental breakage.

## Contract classification

- Human-readable output is best-effort stable. Scripts must use documented
  JSON. Deliberate structural/default/exit-status changes still require review.
- Documented JSON fields may be added when the object is documented as
  extensible. Removing, renaming, changing types/meaning, or changing envelope
  and exit semantics is breaking.
- SQLite index `PRAGMA user_version`, public SQL schema, snapshot Parquet
  schema, gate-log schema, and benchmark schemas must be bumped for
  incompatible persisted changes. Migration or a clear rebuild path is
  required.
- Configuration additions with compatible defaults are minor. Removing keys,
  changing meaning, or changing precedence/default behavior incompatibly is
  breaking.
- MCP tool names, arguments, result schemas, protocol behavior, generated
  plugin manifests/hooks, and compatibility floors are machine contracts.
- Shell completion text itself is not stable, but command/option availability
  and accepted values are. Removing completion-visible surfaces is breaking.
- Package-manager names, archive names/contents, supported targets, checksums,
  crates.io identity/features, and installation commands are contracts.
- `ctx self-update` tag parsing (`v<semver>`), artifact mapping, checksum
  verification, target support, downgrade behavior, and exit codes are
  contracts.
- Exit code 0/1/2/3 meanings are stable. Reassignment or command-specific
  divergence is breaking.

The CLI snapshot and Rust API compatibility job detect a useful subset of
breaks. Changes to sensitive paths require explicit contract review because
machines cannot reliably classify behavioral compatibility. See
`guardrails.md`; never claim that semantic behavior is fully automatic.

## Version operations

```bash
python3 scripts/version.py show
cargo build --locked --release --no-default-features
python3 scripts/version.py check --binary target/release/ctx
python3 scripts/version.py bump patch
python3 scripts/version.py bump minor
python3 scripts/version.py set 0.4.0
```

Mutation commands refuse a dirty tree, invalid/regressing versions, and empty
Unreleased notes unless an explicit override is supplied. They update the
manifest, both lockfiles, release heading/date, and comparison links, print
every changed file, and never commit, tag, push, publish, or access the
network. Re-running the current version is a no-op.

Raising `rust-version` is a compatibility decision. Before 1.0 it requires at
least a minor release and contract review; after 1.0 it requires a major
release unless the project has announced a time-based MSRV policy in this file.
No such exception currently exists.

Internal policy belongs in `governance/`; `docs/` remains the public product
manual.
