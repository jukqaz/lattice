# Change Policy

English | [한국어](change-policy.ko.md) | [Documentation Index](../README.md)

This policy guides changes on the stable line. It keeps Lattice's public surface
stable without blocking small safety and documentation fixes.

## Compatibility Promise

For the stable line, Lattice should preserve:

- documented command names and subcommands;
- documented config keys and default XDG path behavior;
- documented JSON top-level keys for automation surfaces;
- restore safety behavior: dry runs, conflict checks, snapshots, and undo;
- secret handling behavior that stores references and does not read values.

Breaking changes require a migration note, changelog entry, and an explicit
release decision. Unsafe behavior can be removed faster, but the release notes
must explain the safety reason.

## Versioning Rules

- Patch releases fix bugs, docs, release tooling, and compatibility regressions.
- Minor releases may add opt-in commands, optional JSON fields, or additive config
  keys.
- Major releases may remove or reshape stable contracts after deprecation and
  migration guidance.

## JSON Compatibility

Automation users should be able to depend on documented top-level fields. Add new
fields as optional whenever possible. Do not rename or remove documented fields in
patch releases. If a field is best-effort or unstable, label it in the JSON
reference before the release that introduces the field.

## Safety Regression Policy

Safety regressions block release. Do not weaken these gates to make packaging or
public onboarding easier:

- restore dry-run and conflict checks;
- snapshots before forced overwrite;
- traversal, symlink escape, and portable path collision guards;
- secret-looking content checks and non-disclosing secret references;
- metadata-loss warnings.

## Release Approval

Tags and GitHub Releases should be created only after explicit approval for the
release step. Before tagging, run the documented local gates, verify GitHub
Actions, and record install-smoke evidence in the release/Linear closeout.
