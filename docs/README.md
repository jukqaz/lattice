# Lattice Documentation

English | [한국어](README.ko.md) | [Repository README](../README.md)

This directory keeps public documentation separate from LLM-oriented agent
guidance. Public docs come first because they are the main entrypoint for
people evaluating or using Lattice.

## Recommended Reading Order

1. [User Guide](user/usage.md): install Lattice, create services, make the first
   backup, restore safely, and sync service repos with Git.
2. [Product Scope](product/mvp-scope.md): understand what Lattice intentionally
   does, what it avoids, and what belongs in the current release.
3. [Migration Guide](user/migration.md): upgrade from `v0.8.1` to the
   public-stable release line.
4. [Stability Reference](reference/stability.md): review the stable command,
   config, JSON, safety, and deprecation contract.
5. [JSON Output Reference](reference/json-output.md): inspect the machine
   contracts used by scripts and agents.
6. [Change Policy](dev/change-policy.md): review compatibility, safety, and
   release policy on the stable line.
7. [Changelog](../CHANGELOG.md): review release-by-release behavior changes and
   migration notes.
8. [Repository README](../README.md): use the root README as the quick command
   reference.

## Public Docs

| Priority | Document | Use it for |
| --- | --- | --- |
| 1 | [User Guide](user/usage.md) | Day-one setup and common operations |
| 2 | [Product Scope](product/mvp-scope.md) | Product boundaries and release scope |
| 3 | [Migration Guide](user/migration.md) | Upgrade and rollback guidance |
| 4 | [Stability Reference](reference/stability.md) | Stable public CLI contract |
| 5 | [JSON Output Reference](reference/json-output.md) | Machine-readable output contracts |
| 6 | [Change Policy](dev/change-policy.md) | Compatibility, safety, and release policy |
| 7 | [Quality Gates](dev/quality.md) | Local verification and release quality tools |
| 8 | [Changelog](../CHANGELOG.md) | Upgrade and release history |
| 9 | [Korean User Guide](user/usage.ko.md) | Korean day-one setup |
| 10 | [Korean Product Scope](product/mvp-scope.ko.md) | Korean product boundaries |
| 11 | [Korean Migration Guide](user/migration.ko.md) | Korean upgrade and rollback guidance |
| 12 | [Korean Stability Reference](reference/stability.ko.md) | Korean stable public CLI contract |
| 13 | [Korean JSON Output Reference](reference/json-output.ko.md) | Korean machine-readable output contracts |
| 14 | [Korean Change Policy](dev/change-policy.ko.md) | Korean compatibility, safety, and release policy |
| 15 | [Korean Quality Gates](dev/quality.ko.md) | Korean local verification and quality tools |
| 16 | [Korean Changelog](../CHANGELOG.ko.md) | Korean release history |

## LLM Docs

LLM docs are English-only because they are execution rules for coding agents,
not user onboarding material.

| Document | Use it for |
| --- | --- |
| [LLM Documentation Index](llm/README.md) | Agent-facing repository guidance |
| [Branch And Release Policy](llm/branch-release-policy.md) | Commit, PR, CI, tag, and release rules |
| [Kanban Workflow](llm/kanban-workflow.md) | Sequential card handling, recovery-board rules, and completion evidence |

## Language Links

- Korean documentation index: [docs/README.ko.md](README.ko.md)
- Korean root README: [README.ko.md](../README.ko.md)
