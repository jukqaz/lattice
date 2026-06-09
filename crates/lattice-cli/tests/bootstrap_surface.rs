use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_json, run_ok, write_file};

#[test]
fn bootstrap_check_preserves_io_errors_for_unreadable_roots() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let locked_root = temp.path().join("blocked-bootstrap-root");
    symlink(&locked_root, &locked_root).expect("create self-referential bootstrap root symlink");
    fs::write(
        env.config.join("lattice/services/blocked-bootstrap.toml"),
        format!(
            r#"name = "blocked-bootstrap"
root = "{}"
include = ["config.toml"]
"#,
            locked_root.display()
        ),
    )
    .expect("write blocked bootstrap service");

    let output = Command::new(bin)
        .args(["bootstrap", "check"])
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.config)
        .env("XDG_DATA_HOME", &env.data)
        .env("XDG_STATE_HOME", &env.state)
        .env("XDG_CACHE_HOME", &env.cache)
        .output()
        .expect("run command");

    assert!(
        !output.status.success(),
        "bootstrap check unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to inspect service root"));
}

#[test]
fn bootstrap_diagnostics_and_restore_plan_are_trustworthy() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    let init = run_ok(bin, &env, &["init", "--force"]);
    assert!(init.contains("next steps:"));
    assert!(init.contains("lattice bootstrap check"));
    assert!(init.contains("lattice plan <service>"));

    let missing_root = temp.path().join("missing-root");
    let disconnected_repo = temp.path().join("disconnected-repo");
    fs::write(
        env.config.join("lattice/services/disconnected.toml"),
        format!(
            r#"
name = "disconnected"
root = "{}"
repo = "{}"
include = ["config.toml"]
"#,
            missing_root.display(),
            disconnected_repo.display()
        ),
    )
    .expect("write disconnected service config");

    let bootstrap = run_json(bin, &env, &["bootstrap", "check", "--json"]);
    assert_eq!(bootstrap["ok"], false);
    assert_eq!(bootstrap["diagnostics"]["git"], "available");
    assert_eq!(bootstrap["services"][0]["root_exists"], false);
    assert_eq!(bootstrap["services"][0]["repo_exists"], false);
    assert_eq!(bootstrap["services"][0]["git_repo"], false);
    assert_eq!(bootstrap["services"][0]["remote"], "missing");
    assert_eq!(bootstrap["services"][0]["dirty"], false);
    assert!(
        bootstrap["services"][0]["issues"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("missing_root"))
    );
    assert!(
        bootstrap["services"][0]["issues"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("missing_repo"))
    );
    assert!(
        bootstrap["next_actions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "create or restore missing service roots"
            ))
    );

    fs::remove_file(env.config.join("lattice/services/disconnected.toml"))
        .expect("remove disconnected config");
    let source = temp.path().join("plan-source");
    let repo = temp.path().join("plan-repo");
    write_file(&source, "config.toml", "stable = true\n", 0o600);
    fs::write(
        env.config.join("lattice/services/plan.toml"),
        format!(
            r#"
name = "plan"
root = "{}"
repo = "{}"
include = ["config.toml"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write plan config");
    run_ok(bin, &env, &["backup", "plan"]);
    let bootstrap_ready = run_json(bin, &env, &["bootstrap", "check", "--json"]);
    assert_eq!(bootstrap_ready["ok"], true);
    assert_eq!(bootstrap_ready["ready_services"], 1);
    assert!(
        bootstrap_ready["services"][0]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        bootstrap_ready["services"][0]["warnings"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("repo_not_git"))
    );
    fs::write(source.join("config.toml"), "local drift\n").expect("write local drift");

    let plan = run_json(bin, &env, &["plan", "--json", "plan"]);
    assert_eq!(plan["ready"], false);
    assert_eq!(plan["safe_to_restore_without_force"], false);
    assert_eq!(plan["requires_force"], true);
    assert_eq!(
        plan["snapshot_policy"],
        "forced restore snapshots conflicts before overwrite"
    );
    assert_eq!(plan["conflicts"], serde_json::json!(["config.toml"]));

    let dry_restore = run_json(bin, &env, &["restore", "--dry-run", "--json", "plan"]);
    assert_eq!(dry_restore["safe_to_restore_without_force"], false);
    assert_eq!(dry_restore["requires_force"], true);
    assert_eq!(
        dry_restore["snapshot_policy"],
        "forced restore snapshots conflicts before overwrite"
    );
    assert_eq!(dry_restore["conflicts"], serde_json::json!(["config.toml"]));
}
