# LLM Documentation

[Documentation Index](../README.md) | [Repository README](../../README.md)

This directory contains machine-oriented guidance for LLM coding agents working
on Lattice.

These files are not user-facing product documentation. They define how agents
should work in this repository: branch naming, release gates, verification,
review evidence, Kanban sequencing, and safety boundaries.

## Read Order

1. `docs/llm/branch-release-policy.md` for branch, PR, CI, tag, and release
   policy.
2. `docs/llm/kanban-workflow.md` for sequential card handling, one active card at a time, Hermes Kanban recovery board handling, and completion evidence.
3. `README.md` for product behavior and user-facing commands.
4. `docs/user/usage.md` for public user-facing usage.
5. `docs/product/mvp-scope.md` for product scope and non-goals.

## Agent Workflow Docs

| Document | Use it for |
| --- | --- |
| [Branch And Release Policy](branch-release-policy.md) | Commit, PR, CI, tag, and release rules |
| [Kanban Workflow](kanban-workflow.md) | Sequential card handling, recovery-board rules, and completion evidence |

## Nix/NixOS Install Boundary For Agents

When an agent needs Lattice on a Nix or NixOS host, do not perform the human
user install. Agents must not mutate the user's profile, `~/.cargo/bin`, shell
configuration, or Home Manager activation state. Use a task-scoped temporary
install root instead:

```bash
tmp="$(mktemp -d)"
CARGO_HOME="$tmp/cargo-home" \
  nix shell nixpkgs#cargo nixpkgs#rustc -c \
    cargo install --git https://github.com/jukqaz/lattice lattice --tag v1.0.0 --locked --root "$tmp/install"
"$tmp/install/bin/lattice" --version
```

Human users on Nix/NixOS may use the persistent user install documented in the
README and user guide. Agents should only suggest that path to the user; they
should not execute it on the user's behalf unless explicitly approved.

## Boundaries

- Keep LLM guidance in English.
- Keep public user documentation bilingual, with English as the default and
  Korean translations as `.ko.md` sibling files.
- Keep public user documentation focused on installation, usage, safety, and
  support.
- Do not move product instructions into LLM policy docs unless they are
  execution rules for coding agents.
- Do not publish, tag, push, or create releases without explicit user approval.
