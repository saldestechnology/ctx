# Release procedure

Releases are constructed entirely from repository state and require no hidden
local files. Archives are not claimed to be byte-for-byte reproducible across
runner images. The release source is a reviewed commit on `main`; the trigger
is an annotated `v<version>` tag. CI requires the peeled tag commit to be in
`origin/main`. The workflow does not accept release branches or a manual
version input.

## Prepare a release pull request

1. Ensure every user-visible change has a categorized Unreleased changelog
   entry. Allowed categories are Added, Changed, Deprecated, Removed, Fixed,
   Security, Documentation, and Internal. Put `BREAKING:` first in every
   incompatible entry and include migration instructions.
2. Keep embargoed vulnerability detail out of the public changelog until the
   coordinated disclosure point. Use a neutral security entry if necessary.
3. Run `python3 scripts/version.py bump patch|minor|major`. The command moves
   Unreleased notes into a dated release section and updates both lockfiles.
4. Apply the `release-preparation` label. Apply `breaking-change` and
   `contract-review` when applicable.
5. Run `scripts/release-check.sh v$(python3 scripts/version.py show)` and review
   the generated `release-notes.md` without committing that generated file.
6. Merge only after all required checks and CODEOWNERS review pass.

Changelog bullets should link PRs and preserve contributor credit when
practical. An empty release is rejected unless `--allow-empty` is deliberately
used and justified in the release PR.

## Tag and publish

From the reviewed `main` commit:

```bash
version="$(python3 scripts/version.py show)"
git tag -a "v$version" -m "ctx $version"
git push origin "v$version"
```

Signed tags are recommended and are a human-review requirement until the
repository has a maintained signing-key policy; CI currently enforces the
name/content relationship, not cryptographic tag signatures.

The `publish` job targets the protected GitHub environment named `release`.
Administrators must configure required release-maintainer reviewers for that
environment. The repository currently uses a scoped crates.io token; migrate
to crates.io trusted publishing when it is configured and verified for this
repository.

The tag workflow repeats governance, formatting, Clippy, all-feature and
minimal-feature tests, CLI/Rust compatibility checks, publish dry-runs, plugin
lockstep checks, and dependency policy before publishing. It derives reviewed
GitHub release notes from `CHANGELOG.md`, not commit-title heuristics. It builds
four target archives, creates per-archive and aggregate SHA-256 checksums,
publishes `agentis-ctx`, waits for that publication, then creates the GitHub
Release and provenance attestations.

If crate publication succeeds but GitHub Release creation fails, rerun the
failed workflow jobs; do not create a second tag or bump the version. Cargo
publication is immutable. Never move or reuse a published tag.

## Post-release verification

- Verify crates.io shows the exact version and expected feature set.
- Verify every documented platform archive, both plugin archives,
  `SHA256SUMS`, and provenance attestations exist.
- Download one archive, verify its checksum, and run `ctx --version`.
- Verify `ctx self-update --version <version>` on a supported platform.
- Confirm the GitHub notes match the reviewed changelog and no embargoed detail
  was disclosed.
- Leave a fresh empty Unreleased heading; `version.py` already creates it.

Package-manager manifests are not currently committed. If introduced, they
must be generated or added to `version.py check` before being described as
release-supported.

This is maintainer policy under `governance/`; public usage documentation stays
under `docs/`.
