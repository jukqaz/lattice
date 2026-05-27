# Lattice Kanban Workflow For LLM Agents

[LLM Documentation Index](README.md) | [Branch And Release Policy](branch-release-policy.md) |
[Documentation Index](../README.md)

This document is the agent-facing Kanban workflow for Lattice repository work.
It is not user-facing product documentation.

## Defaults

- Work one active card at a time. Do not batch unrelated cards into one diff.
- Keep task order explicit: complete the current card before unblocking exactly the next card.
- Keep `HERMES_HOME=/opt/data` for Hermes Kanban commands in the host-native
  environment used by this repo.
- Run Rust/Cargo verification with `HOME=/opt/data/home` unless the task is
  explicitly a live HOME dogfood check.
- Use English Conventional Commits if a commit is requested, but answer the user
  in Korean.

## Normal Card Lifecycle

1. Inspect the current Kanban card and any parent/blocker notes.
2. Inspect existing code, tests, docs, and release notes before editing.
3. Add or extend the smallest focused regression first when behavior changes.
4. Run the focused test and confirm it fails for the expected reason.
5. Implement the smallest safe change.
6. Re-run the focused test, then run the relevant shared gate:

```bash
HOME=/opt/data/home cargo run -p xtask -- verify
git diff --check

# Use this equivalent if fsmonitor makes local Git status noisy:
git -c core.fsmonitor=false diff --check
```

7. Summarize changed files and exact verification results in the card completion.
8. Complete the current card, then unblock exactly the next card in the sequence.

## Recovery Board Rule

If the active Hermes Kanban board becomes unreadable, corrupted, or otherwise
unusable, create a Hermes Kanban recovery board rather than hand-editing the
broken database. The recovery board must preserve the original card order,
blockers, workspace, and verification expectations. Mention the recovery board
name in handoffs so future agents do not switch back to stale board state.

## Safety Boundaries

- Do not run mutating live HOME commands (`init`, `backup`, `restore`, `adopt`,
  `track`, `snapshot prune`, `undo --yes`, repo commit/push) unless the user
  explicitly approves that live HOME mutation.
- For live HOME dogfood, use only read-only diagnostics such as `doctor`,
  `validate`, `bootstrap check`, `service list`, `discover --json`, and
  `group list --json`.
- Do not publish, push, tag, or create GitHub releases unless the user explicitly
  asks for that side effect.
- Do not print secret values. If a command output contains secrets, redact them
  before adding a card comment or user summary.

## Completion Evidence

A card completion should include:

- the product or safety behavior changed;
- files changed at a high level;
- focused tests run;
- shared gates run;
- any skipped gates and the concrete reason;
- the next card unblocked, if any.

If a task cannot be finished safely, block it with one actionable decision or
missing prerequisite instead of marking it complete.
