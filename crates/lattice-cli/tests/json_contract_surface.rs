use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, assert_json_keys, run_json, run_ok, write_file};

#[test]
fn json_contract_surfaces_have_stable_top_level_keys() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("shell-source");
    let repo = temp.path().join("shell-repo");
    write_file(&source, "config.toml", "prompt = \"compact\"\n", 0o600);
    write_file(&source, "bin/tool", "#!/usr/bin/env sh\n", 0o700);
    fs::write(
        env.config.join("lattice/lattice.toml"),
        r#"version = 1
profile = "main"

[[groups]]
name = "dev-shell"
description = "Shell configuration"
services = ["shell"]
"#,
    )
    .expect("write global config with group");
    fs::write(
        env.config.join("lattice/services/shell.toml"),
        format!(
            r#"name = "shell"
root = "{}"
repo = "{}"
include = ["config.toml", "bin/**"]

[restore]
create_dirs = [
  {{ path = "cache", mode = "0700" }},
]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write shell service config");

    let bootstrap = run_json(bin, &env, &["bootstrap", "check", "--json"]);
    assert_json_keys(
        &bootstrap,
        &[
            "config",
            "config_exists",
            "diagnostics",
            "git",
            "next_actions",
            "ok",
            "ready_services",
            "services",
            "services_dir",
            "services_dir_exists",
        ],
    );

    let status = run_json(bin, &env, &["status", "--json", "shell"]);
    assert_json_keys(
        &status,
        &[
            "active",
            "files",
            "inactive_reasons",
            "included_files",
            "manifest",
            "repo",
            "root",
            "service",
        ],
    );

    let plan = run_json(bin, &env, &["plan", "--json", "shell"]);
    assert_json_keys(
        &plan,
        &[
            "active",
            "backup_would_copy",
            "conflicts",
            "dirs",
            "entries",
            "inactive_reasons",
            "manifest",
            "ready",
            "repo",
            "requires_force",
            "restore_would_create_dirs",
            "restore_would_restore",
            "root",
            "root_exists",
            "safe_to_restore_without_force",
            "service",
            "snapshot_on_conflict",
            "snapshot_policy",
        ],
    );

    let dry_backup = run_json(bin, &env, &["backup", "--dry-run", "--json", "shell"]);
    assert_json_keys(
        &dry_backup,
        &[
            "destination",
            "dirs",
            "dry_run",
            "files",
            "hooks",
            "service",
            "would_copy",
            "would_track_dirs",
        ],
    );

    let group_list = run_json(bin, &env, &["group", "list", "--json"]);
    assert_json_keys(&group_list, &["groups"]);
    let context = run_json(bin, &env, &["context", "show", "--json"]);
    assert_json_keys(&context, &["contexts", "hostname", "os", "profile"]);
    let group_status = run_json(bin, &env, &["group", "status", "--json", "dev-shell"]);
    assert_json_keys(
        &group_status,
        &[
            "active_services",
            "description",
            "group",
            "included_files",
            "service_count",
            "services",
        ],
    );

    run_ok(bin, &env, &["backup", "shell"]);
    fs::write(source.join("config.toml"), "local drift\n").expect("create local drift");

    let diff = run_json(bin, &env, &["diff", "--json", "shell"]);
    assert_json_keys(&diff, &["diffs", "service"]);

    let dry_restore = run_json(bin, &env, &["restore", "--dry-run", "--json", "shell"]);
    assert_json_keys(
        &dry_restore,
        &[
            "conflicts",
            "destination",
            "dirs",
            "dry_run",
            "entries",
            "hooks",
            "requires_force",
            "safe_to_restore_without_force",
            "service",
            "snapshot_policy",
            "would_create_dirs",
            "would_restore",
        ],
    );

    let group_plan = run_json(bin, &env, &["group", "plan", "--json", "dev-shell"]);
    assert_json_keys(
        &group_plan,
        &[
            "active_services",
            "backup_would_copy",
            "conflict_count",
            "conflicts",
            "description",
            "group",
            "ready",
            "restore_would_create_dirs",
            "restore_would_restore",
            "service_count",
            "services",
        ],
    );

    run_ok(bin, &env, &["restore", "--force", "shell"]);

    let snapshot_list = run_json(bin, &env, &["snapshot", "list", "--json"]);
    assert_json_keys(&snapshot_list, &["snapshots"]);
    let snapshot_id = snapshot_list["snapshots"][0]["id"]
        .as_str()
        .expect("snapshot id");

    let snapshot_show = run_json(bin, &env, &["snapshot", "show", "--json", snapshot_id]);
    assert_json_keys(
        &snapshot_show,
        &["entries", "files", "id", "path", "service"],
    );

    let undo = run_json(bin, &env, &["undo", "--dry-run", "--json", snapshot_id]);
    assert_json_keys(
        &undo,
        &[
            "destination",
            "dry_run",
            "entries",
            "preflight",
            "service",
            "snapshot",
            "would_restore",
        ],
    );

    let prune = run_json(
        bin,
        &env,
        &["snapshot", "prune", "--dry-run", "--json", "--keep", "20"],
    );
    assert_json_keys(&prune, &["dry_run", "keep", "remove", "would_remove"]);

    let discovery = run_json(bin, &env, &["discover", "--json"]);
    assert_json_keys(
        &discovery,
        &["mutated", "next_actions", "services_dir", "suggestions"],
    );
}
