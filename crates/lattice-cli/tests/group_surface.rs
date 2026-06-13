use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

use tempfile::tempdir;

mod support;

use support::{TestEnv, assert_json_keys, run_fail, run_json, run_ok, write_file};

#[test]
fn service_group_commands_are_read_only_and_aggregate_status_plan() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let shell_source = temp.path().join("shell-source");
    let git_source = temp.path().join("git-source");
    write_file(
        &shell_source,
        "config.toml",
        "prompt = \"compact\"\n",
        0o600,
    );
    write_file(&git_source, "config", "name = \"Lattice Test\"\n", 0o600);

    let missing_root = temp.path().join("missing-root");
    fs::write(
        env.config.join("lattice/lattice.toml"),
        r#"version = 1
profile = "main"

[[groups]]
name = "dev-shell"
description = "Shell and Git configuration"
services = ["shell", "git", "missing"]
"#,
    )
    .expect("write global config with group and missing service");
    fs::write(
        env.config.join("lattice/services/shell.toml"),
        format!(
            r#"name = "shell"
root = "{}"
include = ["config.toml"]
"#,
            shell_source.display()
        ),
    )
    .expect("write shell service");
    fs::write(
        env.config.join("lattice/services/git.toml"),
        format!(
            r#"name = "git"
root = "{}"
include = ["config"]
"#,
            git_source.display()
        ),
    )
    .expect("write git service");
    fs::write(
        env.config.join("lattice/services/missing.toml"),
        format!(
            r#"name = "missing"
root = "{}"
include = ["config.toml"]
"#,
            missing_root.display()
        ),
    )
    .expect("write missing service");

    let list = run_ok(bin, &env, &["group", "list"]);
    assert!(list.contains("dev-shell services=3"));
    let list_json = run_json(bin, &env, &["group", "list", "--json"]);
    assert_json_keys(&list_json, &["groups"]);
    assert_eq!(list_json["groups"].as_array().unwrap().len(), 1);
    assert_json_keys(
        &list_json["groups"][0],
        &["description", "name", "services"],
    );
    assert_eq!(list_json["groups"][0]["name"], "dev-shell");

    let show = run_ok(bin, &env, &["group", "show", "dev-shell"]);
    assert!(show.contains("group: dev-shell"));
    assert!(show.contains("description: Shell and Git configuration"));
    assert!(show.contains("- shell"));
    assert!(show.contains("- git"));
    assert!(show.contains("- missing"));
    let show_json = run_json(bin, &env, &["group", "show", "--json", "dev-shell"]);
    assert_json_keys(&show_json, &["description", "name", "services"]);
    assert_eq!(show_json["description"], "Shell and Git configuration");
    assert_eq!(show_json["services"].as_array().unwrap().len(), 3);

    let status = run_ok(bin, &env, &["group", "status", "dev-shell"]);
    assert!(status.contains("group: dev-shell"));
    assert!(status.contains("services: 3"));
    assert!(status.contains("included files: 2"));
    assert!(
        status.contains("- shell active=yes root_exists=yes included_files=1 manifest=missing")
    );
    assert!(status.contains("- git active=yes root_exists=yes included_files=1 manifest=missing"));
    assert!(
        status.contains("- missing active=yes root_exists=no included_files=0 manifest=missing")
    );

    let status_json = run_json(bin, &env, &["group", "status", "--json", "dev-shell"]);
    assert_json_keys(
        &status_json,
        &[
            "active_services",
            "description",
            "group",
            "included_files",
            "service_count",
            "services",
        ],
    );
    assert_eq!(status_json["group"], "dev-shell");
    assert_eq!(status_json["included_files"], 2);
    assert_eq!(status_json["services"].as_array().unwrap().len(), 3);
    assert_json_keys(
        &status_json["services"][0],
        &[
            "active",
            "files",
            "inactive_reasons",
            "included_files",
            "manifest",
            "repo",
            "root",
            "root_exists",
            "service",
        ],
    );
    assert_eq!(status_json["services"][0]["service"], "shell");
    assert_eq!(status_json["services"][2]["service"], "missing");
    assert_eq!(status_json["services"][2]["root_exists"], false);
    assert_eq!(status_json["services"][2]["included_files"], 0);

    run_ok(bin, &env, &["backup", "shell"]);
    write_file(
        &shell_source,
        "config.toml",
        "prompt = \"expanded\"\n",
        0o600,
    );
    let plan_json = run_json(bin, &env, &["group", "plan", "--json", "dev-shell"]);
    assert_json_keys(
        &plan_json,
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
    assert_eq!(plan_json["group"], "dev-shell");
    assert_eq!(plan_json["backup_would_copy"], 2);
    assert_eq!(plan_json["restore_would_restore"], 1);
    assert_eq!(plan_json["conflict_count"], 1);
    assert_eq!(plan_json["conflicts"].as_array().unwrap().len(), 1);
    assert_json_keys(
        &plan_json["services"][0],
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
    assert_json_keys(&plan_json["conflicts"][0], &["paths", "service"]);
    assert_eq!(plan_json["conflicts"][0]["service"], "shell");
    assert_eq!(plan_json["ready"], false);
    assert_eq!(plan_json["services"].as_array().unwrap().len(), 3);

    let excluded = run_json(
        bin,
        &env,
        &[
            "group",
            "status",
            "--json",
            "--exclude",
            "config.toml",
            "dev-shell",
        ],
    );
    assert_eq!(excluded["included_files"], 1);

    let unknown = run_fail(bin, &env, &["group", "show", "missing"]);
    assert!(unknown.contains("unknown group missing"));
}

#[test]
fn group_config_validation_rejects_ambiguous_or_broken_groups() {
    let bin = env!("CARGO_BIN_EXE_lattice");
    let cases = [
        (
            r#"version = 1
profile = "main"

[[groups]]
name = "dupe"
services = ["alpha"]

[[groups]]
name = "dupe"
services = ["alpha"]
"#,
            "duplicate group dupe",
        ),
        (
            r#"version = 1
profile = "main"

[[groups]]
name = "empty"
"#,
            "group empty must include at least one service",
        ),
        (
            r#"version = 1
profile = "main"

[[groups]]
name = "broken"
services = ["ghost"]
"#,
            "group broken references unknown service ghost",
        ),
        (
            r#"version = 1
profile = "main"

[[groups]]
name = "repeat"
services = ["alpha", "alpha"]
"#,
            "group repeat lists service alpha more than once",
        ),
    ];

    for (global_config, expected_error) in cases {
        let temp = tempdir().expect("tempdir");
        let env = TestEnv::new(temp.path());
        run_ok(bin, &env, &["init", "--force"]);
        let root = temp.path().join("alpha-root");
        write_file(&root, "config.toml", "alpha = true\n", 0o600);
        fs::write(
            env.config.join("lattice/services/alpha.toml"),
            format!(
                r#"name = "alpha"
root = "{}"
include = ["config.toml"]
"#,
                root.display()
            ),
        )
        .expect("write alpha service");
        fs::write(env.config.join("lattice/lattice.toml"), global_config)
            .expect("write invalid group config");

        let validate = run_fail(bin, &env, &["validate"]);
        assert!(
            validate.contains(expected_error),
            "expected {expected_error:?}, got:\n{validate}"
        );
        let group_list = run_fail(bin, &env, &["group", "list"]);
        assert!(
            group_list.contains(expected_error),
            "expected {expected_error:?}, got:\n{group_list}"
        );
    }
}

#[test]
fn group_plan_aggregates_only_active_services() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let active_root = temp.path().join("active-root");
    let inactive_root = temp.path().join("inactive-root");
    write_file(&active_root, "active.toml", "enabled = true\n", 0o600);
    write_file(&inactive_root, "inactive.toml", "enabled = false\n", 0o600);
    fs::write(
        env.config.join("lattice/lattice.toml"),
        r#"version = 1
profile = "main"

[[groups]]
name = "mixed"
services = ["active", "inactive"]
"#,
    )
    .expect("write mixed group config");
    fs::write(
        env.config.join("lattice/services/active.toml"),
        format!(
            r#"name = "active"
root = "{}"
include = ["*.toml"]
"#,
            active_root.display()
        ),
    )
    .expect("write active service");
    fs::write(
        env.config.join("lattice/services/inactive.toml"),
        format!(
            r#"name = "inactive"
root = "{}"
include = ["*.toml"]
[conditions]
os = "not-this-os"
"#,
            inactive_root.display()
        ),
    )
    .expect("write inactive service");

    let status = run_json(bin, &env, &["group", "status", "--json", "mixed"]);
    assert_eq!(status["service_count"], 2);
    assert_eq!(status["active_services"], 1);
    assert_eq!(status["included_files"], 1);
    assert_eq!(status["services"][1]["active"], false);
    assert_eq!(
        status["services"][1]["root_exists"],
        serde_json::Value::Null
    );
    assert_eq!(status["services"][1]["included_files"], 0);

    let plan = run_json(bin, &env, &["group", "plan", "--json", "mixed"]);
    assert_eq!(plan["service_count"], 2);
    assert_eq!(plan["active_services"], 1);
    assert_eq!(plan["backup_would_copy"], 1);
    assert_eq!(plan["services"][1]["backup_would_copy"], 0);
    assert_eq!(plan["services"][1]["root_exists"], serde_json::Value::Null);
}

#[test]
fn group_status_preserves_io_errors_for_unreadable_roots() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let locked_root = temp.path().join("blocked-root");
    symlink(&locked_root, &locked_root).expect("create self-referential root symlink");
    fs::write(
        env.config.join("lattice/lattice.toml"),
        r#"version = 1
profile = "main"

[[groups]]
name = "blocked"
services = ["blocked"]
"#,
    )
    .expect("write blocked group config");
    fs::write(
        env.config.join("lattice/services/blocked.toml"),
        format!(
            r#"name = "blocked"
root = "{}"
include = ["config.toml"]
"#,
            locked_root.display()
        ),
    )
    .expect("write blocked service");

    let output = Command::new(bin)
        .args(["group", "status", "blocked"])
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.config)
        .env("XDG_DATA_HOME", &env.data)
        .env("XDG_STATE_HOME", &env.state)
        .env("XDG_CACHE_HOME", &env.cache)
        .output()
        .expect("run command");

    assert!(
        !output.status.success(),
        "group status unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to inspect service root"));
}
