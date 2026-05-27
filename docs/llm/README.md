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

## Boundaries

- Keep LLM guidance in English.
- Keep public user documentation bilingual, with English as the default and
  Korean translations as `.ko.md` sibling files.
- Keep public user documentation focused on installation, usage, safety, and
  support.
- Do not move product instructions into LLM policy docs unless they are
  execution rules for coding agents.
- Do not publish, tag, push, or create releases without explicit user approval.
