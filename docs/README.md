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
3. [JSON Output Reference](reference/json-output.md): inspect the machine
   contracts used by scripts and agents.
4. [Changelog](../CHANGELOG.md): review release-by-release behavior changes and
   migration notes.
5. [Repository README](../README.md): use the root README as the quick command
   reference.

## Public Docs

| Priority | Document | Use it for |
| --- | --- | --- |
| 1 | [User Guide](user/usage.md) | Day-one setup and common operations |
| 2 | [Product Scope](product/mvp-scope.md) | Product boundaries and release scope |
| 3 | [JSON Output Reference](reference/json-output.md) | Machine-readable output contracts |
| 4 | [Quality Gates](dev/quality.md) | Local verification and release quality tools |
| 5 | [Changelog](../CHANGELOG.md) | Upgrade and release history |
| 6 | [Korean User Guide](user/usage.ko.md) | Korean day-one setup |
| 7 | [Korean Product Scope](product/mvp-scope.ko.md) | Korean product boundaries |
| 8 | [Korean JSON Output Reference](reference/json-output.ko.md) | Korean machine-readable output contracts |
| 9 | [Korean Quality Gates](dev/quality.ko.md) | Korean local verification and quality tools |
| 10 | [Korean Changelog](../CHANGELOG.ko.md) | Korean release history |

## LLM Docs

LLM docs are English-only because they are execution rules for coding agents,
not user onboarding material.

| Document | Use it for |
| --- | --- |
| [LLM Documentation Index](llm/README.md) | Agent-facing repository guidance |
| [Branch And Release Policy](llm/branch-release-policy.md) | Commit, PR, CI, tag, and release rules |

## Language Links

- Korean documentation index: [docs/README.ko.md](README.ko.md)
- Korean root README: [README.ko.md](../README.ko.md)
