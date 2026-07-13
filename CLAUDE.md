# Claude operating instructions for ctx

<!-- governance-instructions:v1 -->

Follow [`AGENTS.md`](AGENTS.md) as the root contributor contract. Read
[`governance/agent-workflow.md`](governance/agent-workflow.md) before editing
and [`governance/versioning.md`](governance/versioning.md) before touching any
compatibility or release surface. `docs/ is public product documentation`;
internal instructions and maintainer policy belong only under `governance/`.

## ctx-assisted development

Prefer the repository index for code intelligence, then verify against source:

- `ctx map --budget 2000`
- `ctx query find <name>` / `ctx search <query>`
- `ctx source <symbol>` / `ctx explain <symbol>`
- `ctx similar <symbol>` before adding an implementation
- `ctx check` and `ctx score --against <base>` before handoff

Do not edit generated `.claude/hooks/ctx/`, `.codex/` harness files, plugin
manifests, release notes, lockfile versions, or CLI contract snapshots by hand.
Use their repository scripts. Never bypass a compatibility/version/security
gate or describe a human-review rule as automatically enforced.

## Project tracking

Work is tracked in Linear under team **agentis** (`AGE`), project **ctx**:
https://linear.app/itkonsult/project/ctx-7625ddcc6dcb. New repository issues
belong to that team/project when issue creation is explicitly requested.
