# Migration Guide

English | [한국어](migration.ko.md) | [Documentation Index](../README.md)

This guide explains how to move from the previous `v0.8.1` release line to the
current `v1.0.0` stable CLI without weakening Lattice's safety model.

## Supported Upgrade Shape

Lattice remains Git-tag distributed for the stable line. Install an explicit tag:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v1.0.0 --locked
```

After installing the stable tag, verify:

```bash
lattice --version
lattice validate
lattice doctor
lattice bootstrap check
lattice plan <service>
```

Do not run `restore --force` immediately after upgrading. First run `plan` and
`restore --dry-run` for the service you intend to restore.

## From v0.8.1 To v1.0.0

`v1.0.0` is the stable contract line. It improves public documentation, migration
guidance, change policy, issue/PR templates, and home/work context documentation
without changing the core backup/restore model.

Expected migration impact:

- Existing service TOML files should continue to load.
- Existing backup repos and snapshot history should remain usable.
- JSON top-level contracts should either stay compatible or document any new
  optional fields.
- Context labels are opt-in and additive. Services without context conditions
  behave as before; services with `conditions.contexts` expose skip details in
  `inactive_reasons` on JSON planning surfaces.

## Stable Contract Review

Before upgrading automation or a real HOME workflow, read the changelog and
stability reference for the exact tag.

Recommended upgrade checklist:

1. Install the target tag explicitly.
2. Run `lattice validate` and fix config warnings before any backup or restore.
3. Run `lattice status --json` and `lattice plan --json` for automation that
   depends on machine-readable output.
4. Run one low-risk `backup --dry-run` and inspect the output.
5. Run `restore --dry-run` before any real restore.
6. Keep the previous install command/tag in your notes so rollback is explicit.

Rollback uses the same explicit tag install shape:

```bash
cargo install --git https://github.com/jukqaz/lattice lattice --tag v0.8.1 --locked
```

## Stable At v1.0

The v1.0 contract should cover:

- documented command names and subcommand shapes;
- documented config keys and default path behavior;
- documented JSON top-level keys for automation surfaces;
- dry-run-first safety behavior;
- snapshots before forced overwrite and undo inspection;
- secret references that do not read or materialize secret values.

## Still Out Of Scope

Do not expect the v1.0 stable line to add package installation, remote repo creation,
secret value management, group backup/restore mutation, GUI, database-backed
state, per-file alternates, or full conditional templates.
