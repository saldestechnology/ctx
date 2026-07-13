## Description

Brief description of the changes.

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update

## Compatibility and release impact

- [ ] No compatibility-sensitive contract changed
- [ ] Contract changed additively; I updated the CLI snapshot if applicable
- [ ] Breaking change; the PR has `contract-review` and `breaking-change`
      maintainer acknowledgement, a compatible version bump, migration notes,
      and a prominent `BREAKING:` changelog entry
- [ ] Release preparation; the PR has the `release-preparation` label and used
      `scripts/version.py`

## Checklist

- [ ] I have run `cargo fmt`
- [ ] I have run `cargo clippy` and addressed any warnings
- [ ] I have added tests that prove my fix/feature works
- [ ] I have updated documentation if needed
- [ ] I added a categorized `CHANGELOG.md` Unreleased entry, or a maintainer
      confirmed `skip-changelog` is appropriate
- [ ] I reviewed CLI, JSON, config, schema/index, Rust API, MCP/plugin, exit-code,
      platform/package, and self-update compatibility as applicable
- [ ] I ran `python3 scripts/check-governance.py check`
- [ ] All tests pass (`cargo test`)

## Related Issues

Closes #(issue number)
