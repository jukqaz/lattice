## Summary

-

## Safety / compatibility

- [ ] Does not weaken dry-run-first restore safety, conflict checks, snapshots, undo, secret handling, traversal/symlink guards, or metadata-loss warnings.
- [ ] Documents any command/config/JSON contract change.
- [ ] Keeps group commands read-only unless a future release explicitly changes that scope.

## Verification

- [ ] `cargo +stable fmt --check`
- [ ] `git diff --check`
- [ ] `RUSTUP_TOOLCHAIN=stable scripts/lint.sh`
- [ ] `cargo +stable run -p xtask -- verify`
- [ ] `cargo +stable run -p xtask -- quality`
- [ ] `cargo +stable run -p xtask -- release-check 1.0.0`
