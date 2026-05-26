use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

#[test]
fn cli_help_surfaces_actionable_descriptions() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    let help = run_ok(bin, &env, &["--help"]);
    assert!(help.contains("A small dotfiles and configuration manager"));
    assert!(help.contains("Manage app catalog shortcuts"));
    assert!(help.contains("Check new-machine readiness without mutating state"));
    assert!(help.contains("Summarize backup and restore risk before changing files"));
    assert!(help.contains("Inspect and prune restore safety snapshots"));
    assert!(help.contains("Suggest local service candidates without mutating state"));
    assert!(help.contains("Inspect service groups without mutating state"));

    let app_help = run_ok(bin, &env, &["app", "--help"]);
    assert!(app_help.contains("List built-in app catalog entries"));
    assert!(app_help.contains("Show the suggested config for an app"));
    assert!(app_help.contains("Create a service config from an app catalog entry"));

    let group_help = run_ok(bin, &env, &["group", "--help"]);
    assert!(group_help.contains("List configured service groups"));
    assert!(group_help.contains("Show one service group"));
    assert!(group_help.contains("Show grouped service status"));
    assert!(group_help.contains("Summarize grouped service plans"));

    let snapshot_help = run_ok(bin, &env, &["snapshot", "--help"]);
    assert!(snapshot_help.contains("List recorded restore safety snapshots"));
    assert!(snapshot_help.contains("Show files captured in one safety snapshot"));
    assert!(snapshot_help.contains("Delete old safety snapshots after keeping recent entries"));
}

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
    assert_eq!(list_json["groups"].as_array().unwrap().len(), 1);
    assert_eq!(list_json["groups"][0]["name"], "dev-shell");

    let show = run_ok(bin, &env, &["group", "show", "dev-shell"]);
    assert!(show.contains("group: dev-shell"));
    assert!(show.contains("description: Shell and Git configuration"));
    assert!(show.contains("- shell"));
    assert!(show.contains("- git"));
    assert!(show.contains("- missing"));
    let show_json = run_json(bin, &env, &["group", "show", "--json", "dev-shell"]);
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
    assert_eq!(status_json["group"], "dev-shell");
    assert_eq!(status_json["included_files"], 2);
    assert_eq!(status_json["services"].as_array().unwrap().len(), 3);
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
    assert_eq!(plan_json["group"], "dev-shell");
    assert_eq!(plan_json["backup_would_copy"], 2);
    assert_eq!(plan_json["restore_would_restore"], 1);
    assert_eq!(plan_json["conflict_count"], 1);
    assert_eq!(plan_json["conflicts"].as_array().unwrap().len(), 1);
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
fn init_doctor_list_backup_and_restore_generic_service() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    let version = run_ok(bin, &env, &["--version"]);
    assert!(version.contains("lattice"));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));

    run_ok(bin, &env, &["init", "--force"]);
    assert!(env.config.join("lattice/lattice.toml").exists());
    assert!(env.config.join("lattice/services").is_dir());
    assert!(!env.config.join("lattice/services/codex.toml").exists());

    let doctor = run_ok(bin, &env, &["doctor"]);
    assert!(doctor.contains("config:"));
    assert!(doctor.contains("rbw:"));
    assert!(doctor.contains("bw:"));

    let validate = run_ok(bin, &env, &["validate"]);
    assert!(validate.contains("valid config"));
    assert!(validate.contains("services: 0"));

    let bootstrap = run_ok(bin, &env, &["bootstrap", "check"]);
    assert!(bootstrap.contains("bootstrap check"));
    assert!(bootstrap.contains("ready services: 0"));

    let source = temp.path().join("shell-source");
    let repo = temp.path().join("shell-repo");
    let hook_marker = temp.path().join("after-restore-hook.txt");
    let confirm_marker = temp.path().join("confirm-hook.txt");
    write_file(&source, "config.toml", "prompt = \"compact\"\n", 0o600);
    write_file(&source, "bin/tool", "#!/usr/bin/env bash\n", 0o700);
    write_file(&source, "auth.json", "{}\n", 0o600);
    fs::write(
        env.config.join("lattice/services/shell.toml"),
        format!(
            r#"
name = "shell"
root = "{}"
repo = "{}"
include = ["config.toml", "bin/**"]
exclude = ["auth.json"]

[restore]
create_dirs = [
  {{ path = "cache", mode = "0700" }},
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[permissions]]
path = "bin/tool"
mode = "0700"

[[hooks.after_restore]]
name = "write after restore marker"
command = "/bin/sh"
args = ["-c", "printf after_restore > '{}'" ]
timeout_sec = 30
confirm = false

[[hooks.before_backup]]
name = "confirm required"
command = "/bin/sh"
args = ["-c", "printf confirmed > '{}'" ]
confirm = true
"#,
            source.display(),
            repo.display(),
            hook_marker.display(),
            confirm_marker.display()
        ),
    )
    .expect("write service config");

    let services = run_ok(bin, &env, &["service", "list"]);
    assert!(services.contains("shell"));

    let status = run_ok(bin, &env, &["status", "shell"]);
    assert!(status.contains("service: shell"));
    assert!(status.contains("included files: 2"));
    assert!(status.contains("manifest: missing"));

    let plan_before_backup = run_ok(bin, &env, &["plan", "shell"]);
    assert!(plan_before_backup.contains("plan: shell"));
    assert!(plan_before_backup.contains("manifest: missing"));
    assert!(plan_before_backup.contains("backup would copy: 2"));

    let dry_backup = run_ok(bin, &env, &["backup", "--dry-run", "shell"]);
    assert!(dry_backup.contains("would copy 2 files"));
    assert!(dry_backup.contains("would run hook before_backup: confirm required"));
    assert!(!repo.join("config.toml").exists());
    assert!(!repo.join(".lattice/manifest.toml").exists());
    assert!(!confirm_marker.exists());

    let backup = run_ok(bin, &env, &["backup", "shell"]);
    assert!(backup.contains("copied 2 files"));
    assert!(backup.contains("skipped hook before_backup: confirm required"));
    assert!(repo.join("config.toml").exists());
    assert!(repo.join("bin/tool").exists());
    assert!(!repo.join("auth.json").exists());
    assert!(repo.join(".lattice/manifest.toml").exists());

    let plan_after_backup = run_ok(bin, &env, &["plan", "shell"]);
    assert!(plan_after_backup.contains("manifest: present"));
    assert!(plan_after_backup.contains("restore would restore: 2"));

    fs::write(source.join("config.toml"), "local drift\n").expect("create local drift");
    let dry_restore = run_ok(bin, &env, &["restore", "--dry-run", "shell"]);
    assert!(dry_restore.contains("would restore 2 files"));
    assert!(dry_restore.contains("conflicts: 1"));
    assert!(dry_restore.contains("conflict config.toml"));
    assert!(dry_restore.contains("would run hook after_restore: write after restore marker"));
    assert!(!hook_marker.exists());

    let failed_restore = run_fail(bin, &env, &["restore", "shell"]);
    assert!(failed_restore.contains("restore conflicts"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("local drift"),
        "local drift\n"
    );

    let forced_restore = run_ok(bin, &env, &["restore", "--force", "shell"]);
    assert!(forced_restore.contains("restored 2 files"));
    assert!(forced_restore.contains("snapshot:"));
    assert!(forced_restore.contains("ran hook after_restore: write after restore marker"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("forced restore"),
        "prompt = \"compact\"\n"
    );
    assert_eq!(
        fs::read_to_string(&hook_marker).expect("hook marker"),
        "after_restore"
    );

    let backup_yes = run_ok(bin, &env, &["backup", "--yes", "shell"]);
    assert!(backup_yes.contains("ran hook before_backup: confirm required"));
    assert_eq!(
        fs::read_to_string(&confirm_marker).expect("confirm marker"),
        "confirmed"
    );

    fs::remove_dir_all(&source).expect("remove source");
    let restore = run_ok(bin, &env, &["restore", "shell"]);
    assert!(restore.contains("restored 2 files"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("restored config"),
        "prompt = \"compact\"\n"
    );
    assert_eq!(mode(&source.join("config.toml")), 0o600);
    assert_eq!(mode(&source.join("bin/tool")), 0o700);
    assert!(source.join("cache").is_dir());
    assert_eq!(mode(&source.join("cache")), 0o700);
}

#[test]
fn custom_service_blocks_secret_like_content_unless_allowed() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("shell-source");
    let repo = temp.path().join("shell-repo");
    write_file(&source, ".zshrc", "export EDITOR=vim\n", 0o644);
    write_file(
        &source,
        ".config/tool/config.toml",
        &format!(
            "api_key = \"{}proj_fake_but_token_shaped\"\n",
            ["s", "k-"].concat()
        ),
        0o600,
    );
    write_file(&source, ".config/tool/cache.tmp", "cache\n", 0o644);

    fs::write(
        env.config.join("lattice/services/shell.toml"),
        format!(
            r#"
name = "shell"
root = "{}"
repo = "{}"
include = [".zshrc", ".config/tool/**"]
exclude = [".config/tool/cache.tmp"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write shell service config");

    let services = run_ok(bin, &env, &["service", "list"]);
    assert!(services.contains("shell"));

    let dry_backup = run_ok(bin, &env, &["backup", "--dry-run", "shell"]);
    assert!(dry_backup.contains("would copy 2 files"));
    assert!(dry_backup.contains(".zshrc"));
    assert!(dry_backup.contains(".config/tool/config.toml"));
    assert!(!dry_backup.contains(".config/tool/cache.tmp"));

    let blocked = run_fail(bin, &env, &["backup", "shell"]);
    assert!(blocked.contains("secret-looking content"));
    assert!(!repo.join(".zshrc").exists());

    let allowed = run_ok(
        bin,
        &env,
        &["backup", "--allow-secret-looking-files", "shell"],
    );
    assert!(allowed.contains("copied 2 files"));
    assert!(repo.join(".zshrc").exists());
    assert!(repo.join(".config/tool/config.toml").exists());
    assert!(!repo.join(".config/tool/cache.tmp").exists());
}

#[test]
fn service_without_repo_uses_xdg_data_repo_named_after_service() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("editor-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600);
    fs::create_dir_all(source.join("profiles/empty")).expect("create empty profile");

    fs::write(
        env.config.join("lattice/services/editor.toml"),
        format!(
            r#"
name = "editor"
root = "{}"
include = ["settings.toml", "profiles/**"]
"#,
            source.display()
        ),
    )
    .expect("write editor service config");

    let expected_repo = env.data.join("lattice/repos/editor");
    let status = run_ok(bin, &env, &["status", "editor"]);
    assert!(status.contains(&format!("repo: {}", expected_repo.display())));

    let backup = run_ok(bin, &env, &["backup", "editor"]);
    assert!(backup.contains(&format!("copied 1 files to {}", expected_repo.display())));
    assert!(backup.contains("tracked 1 empty dirs"));
    assert!(expected_repo.join("settings.toml").exists());
    assert!(expected_repo.join("profiles/empty").is_dir());
    assert!(expected_repo.join(".lattice/manifest.toml").exists());

    fs::remove_dir_all(source.join("profiles/empty")).expect("remove empty profile");
    let restore = run_ok(bin, &env, &["restore", "editor"]);
    assert!(restore.contains("created 1 backed-up empty dirs"));
    assert!(source.join("profiles/empty").is_dir());
}

#[test]
fn service_management_commands_create_update_and_remove_service_config() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("editor-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600);
    write_file(&source, "cache.tmp", "cache\n", 0o644);

    let add = run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "editor",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "settings.toml",
            "--exclude",
            "cache.tmp",
        ],
    );
    assert!(add.contains("added service editor"));

    let show = run_ok(bin, &env, &["service", "show", "editor"]);
    assert!(show.contains("name = \"editor\""));
    assert!(show.contains("\"settings.toml\""));
    assert!(show.contains("\"cache.tmp\""));

    let expected_repo = env.data.join("lattice/repos/editor");
    let status = run_ok(bin, &env, &["status", "editor"]);
    assert!(status.contains(&format!("repo: {}", expected_repo.display())));
    assert!(status.contains("included files: 1"));

    run_ok(
        bin,
        &env,
        &["include", "add", "editor", "README.md", "settings.toml"],
    );
    run_ok(
        bin,
        &env,
        &["exclude", "add", "editor", "target/**", "cache.tmp"],
    );
    run_ok(
        bin,
        &env,
        &["permission", "set", "editor", "settings.toml", "0600"],
    );

    let updated = run_ok(bin, &env, &["service", "show", "editor"]);
    assert!(updated.contains("\"README.md\""));
    assert!(updated.contains("\"settings.toml\""));
    assert!(updated.contains("\"cache.tmp\""));
    assert!(updated.contains("\"target/**\""));
    assert!(updated.contains("path = \"settings.toml\""));
    assert!(updated.contains("mode = \"0600\""));

    run_ok(bin, &env, &["include", "remove", "editor", "README.md"]);
    run_ok(bin, &env, &["exclude", "remove", "editor", "target/**"]);
    run_ok(
        bin,
        &env,
        &["permission", "remove", "editor", "settings.toml"],
    );

    let pruned = run_ok(bin, &env, &["service", "show", "editor"]);
    assert!(!pruned.contains("README.md"));
    assert!(!pruned.contains("target/**"));
    assert!(!pruned.contains("mode = \"0600\""));

    let remove_without_yes = run_fail(bin, &env, &["service", "remove", "editor"]);
    assert!(remove_without_yes.contains("requires --yes"));

    let remove = run_ok(bin, &env, &["service", "remove", "--yes", "editor"]);
    assert!(remove.contains("removed service editor"));
    assert!(!env.config.join("lattice/services/editor.toml").exists());
}

#[test]
fn mvp2_commands_cover_apps_repo_secrets_track_adopt_diff_and_tui() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("home");
    write_file(&source, ".zshrc", "export EDITOR=vim\n", 0o644);
    write_file(&source, ".zprofile", "path+=('/opt/homebrew/bin')\n", 0o644);

    let apps = run_ok(bin, &env, &["app", "list"]);
    assert!(apps.contains("codex"));
    assert!(apps.contains("zsh"));
    assert!(apps.contains("ssh"));

    let zsh_app = run_ok(bin, &env, &["app", "show", "zsh"]);
    assert!(zsh_app.contains("app: zsh"));
    assert!(zsh_app.contains(".zshrc"));
    assert!(zsh_app.contains(".zsh_history"));

    run_ok(
        bin,
        &env,
        &[
            "app",
            "add",
            "zsh",
            "--root",
            source.to_str().expect("source path"),
        ],
    );

    let status = run_ok(bin, &env, &["status", "zsh"]);
    assert!(status.contains("included files: 2"));

    run_ok(bin, &env, &["track", "zsh", ".config/starship.toml"]);
    let tracked = run_ok(bin, &env, &["service", "show", "zsh"]);
    assert!(tracked.contains(".config/starship.toml"));

    run_ok(
        bin,
        &env,
        &[
            "secret",
            "add",
            "zsh",
            "github-token",
            "--backend",
            "rbw",
            "--item",
            "GitHub token",
            "--field",
            "password",
            "--env",
            "GITHUB_TOKEN",
        ],
    );
    let secrets = run_ok(bin, &env, &["secret", "list", "zsh"]);
    assert!(secrets.contains("github-token backend=rbw item=GitHub token"));
    assert!(!secrets.contains("password="));
    let secret_check = run_ok(bin, &env, &["secret", "check", "zsh"]);
    assert!(secret_check.contains("value=not-read"));

    let repo = env.data.join("lattice/repos/zsh");
    run_ok(bin, &env, &["backup", "zsh"]);
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "lattice@example.test"]);
    run_git(&repo, &["config", "user.name", "Lattice Test"]);

    let repo_status = run_ok(bin, &env, &["repo", "status", "zsh"]);
    assert!(repo_status.contains("##"));
    run_ok(
        bin,
        &env,
        &["repo", "commit", "zsh", "--message", "initial backup"],
    );

    fs::write(source.join(".zshrc"), "export EDITOR=nvim\n").expect("modify zshrc");
    let diff = run_ok(bin, &env, &["diff", "zsh"]);
    assert!(diff.contains("diff .zshrc"));
    assert!(diff.contains("+export EDITOR=nvim"));

    let adopt = run_ok(bin, &env, &["adopt", "zsh", ".zprofile"]);
    assert!(adopt.contains("copied"));

    fs::write(
        env.config.join("lattice/services/\\.toml"),
        format!(
            r#"
name = "\\"
root = "{}"
include = [".zshrc"]
"#,
            source.display()
        ),
    )
    .expect("write service with invalid default repo name");

    let tui = run_ok(bin, &env, &["tui", "--dry-run"]);
    assert!(tui.contains("lattice tui dashboard"));
    assert!(tui.contains("services:"));
    assert!(tui.contains("- zsh active=yes"));
    assert!(tui.contains("repo=unavailable(service name"));
    assert!(tui.contains("files=2"));
    assert!(tui.contains("repo="));
    assert!(tui.contains("actions:"));
    assert!(tui.contains("backup --dry-run <service>"));
    assert!(tui.contains("plan <service>"));
    assert!(tui.contains("diff <service>"));
}

#[test]
fn mvp2_restore_modes_cover_template_symlink_and_conditions() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let template_source = temp.path().join("template-source");
    write_file(&template_source, "config.txt", "home={{env:HOME}}\n", 0o600);
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "templated",
            "--root",
            template_source.to_str().expect("template source path"),
            "--include",
            "config.txt",
            "--template",
            "--os",
            std::env::consts::OS,
        ],
    );
    run_ok(bin, &env, &["backup", "templated"]);
    fs::remove_dir_all(&template_source).expect("remove template source");
    run_ok(bin, &env, &["restore", "templated"]);
    assert_eq!(
        fs::read_to_string(template_source.join("config.txt")).expect("rendered config"),
        format!("home={}\n", env.home.display())
    );

    let link_source = temp.path().join("link-source");
    write_file(&link_source, "tool.conf", "mode = 'linked'\n", 0o644);
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "linked",
            "--root",
            link_source.to_str().expect("link source path"),
            "--include",
            "tool.conf",
            "--symlink",
        ],
    );
    run_ok(bin, &env, &["backup", "linked"]);
    fs::remove_dir_all(&link_source).expect("remove link source");
    run_ok(bin, &env, &["restore", "linked"]);
    assert!(
        fs::symlink_metadata(link_source.join("tool.conf"))
            .expect("linked file metadata")
            .file_type()
            .is_symlink()
    );

    let template_link_source = temp.path().join("template-link-source");
    write_file(
        &template_link_source,
        "tool.conf",
        "home={{env:HOME}}\n",
        0o600,
    );
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "templated-linked",
            "--root",
            template_link_source
                .to_str()
                .expect("template link source path"),
            "--include",
            "tool.conf",
            "--template",
            "--symlink",
        ],
    );
    run_ok(bin, &env, &["backup", "templated-linked"]);
    fs::remove_dir_all(&template_link_source).expect("remove template link source");
    run_ok(bin, &env, &["restore", "templated-linked"]);
    assert!(
        !fs::symlink_metadata(template_link_source.join("tool.conf"))
            .expect("templated linked metadata")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(template_link_source.join("tool.conf")).expect("templated linked file"),
        format!("home={}\n", env.home.display())
    );
    let clean_template_diff = run_ok(bin, &env, &["diff", "templated-linked"]);
    assert!(clean_template_diff.trim().is_empty());
    fs::write(template_link_source.join("tool.conf"), "home=changed\n")
        .expect("modify rendered template file");
    let changed_template_diff = run_ok(bin, &env, &["diff", "templated-linked"]);
    assert!(changed_template_diff.contains("template-rendered content differs"));
    assert!(!changed_template_diff.contains("home=changed"));

    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "inactive",
            "--root",
            template_source.to_str().expect("inactive source path"),
            "--include",
            "config.txt",
            "--os",
            "__never__",
        ],
    );
    let inactive_status = run_ok(bin, &env, &["status", "inactive"]);
    assert!(inactive_status.contains("active: no"));
    let inactive_backup = run_fail(bin, &env, &["backup", "inactive"]);
    assert!(inactive_backup.contains("inactive on this host"));
}

#[test]
fn cli_failure_harness_covers_invalid_inputs_permissions_and_noninteractive_tui() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    assert_eq!(mode(&env.config.join("lattice/lattice.toml")), 0o600);
    assert!(env.config.join("lattice/services").is_dir());

    fs::write(env.config.join("lattice/lattice.toml"), "version =\n")
        .expect("write invalid global config");
    let invalid_global = run_fail(bin, &env, &["validate"]);
    assert!(invalid_global.contains("failed to parse"));

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("editor-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600);

    let bad_service_name = run_fail(
        bin,
        &env,
        &[
            "service",
            "add",
            "../escape",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "settings.toml",
        ],
    );
    assert!(bad_service_name.contains("cannot be used as a default repo directory"));

    let unknown_app = run_fail(
        bin,
        &env,
        &[
            "app",
            "add",
            "__missing__",
            "--root",
            source.to_str().expect("source path"),
        ],
    );
    assert!(unknown_app.contains("unknown app"));

    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "editor",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "settings.toml",
        ],
    );
    assert_eq!(
        mode(&env.config.join("lattice/services/editor.toml")),
        0o600
    );

    let bad_mode = run_fail(
        bin,
        &env,
        &["permission", "set", "editor", "settings.toml", "9999"],
    );
    assert!(bad_mode.contains("invalid mode") || bad_mode.contains("mode must"));

    let tui = run_fail(bin, &env, &["tui"]);
    assert!(tui.contains("interactive TUI requires a terminal"));
}

#[test]
fn config_failure_harness_rejects_name_mismatch_and_unsafe_permission_paths() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("mismatch-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600);
    fs::write(
        env.config.join("lattice/services/mismatch.toml"),
        format!(
            r#"
name = "different"
root = "{}"
include = ["settings.toml"]
"#,
            source.display()
        ),
    )
    .expect("write mismatched service config");

    let invalid = run_fail(bin, &env, &["validate"]);
    assert!(invalid.contains("service config name mismatch"));

    fs::remove_file(env.config.join("lattice/services/mismatch.toml"))
        .expect("remove mismatched service");
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "editor",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "settings.toml",
        ],
    );

    let unsafe_permission = run_fail(
        bin,
        &env,
        &["permission", "set", "editor", "../secret.txt", "0600"],
    );
    assert!(unsafe_permission.contains("unsafe relative path"));
    let service = run_ok(bin, &env, &["service", "show", "editor"]);
    assert!(!service.contains("../secret.txt"));
}

#[test]
fn filesystem_edge_harness_rejects_overlap_names_and_metadata_loss() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let overlap_source = temp.path().join("overlap-source");
    write_file(&overlap_source, "config.toml", "source\n", 0o600);
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "overlap",
            "--root",
            overlap_source.to_str().expect("overlap root"),
            "--repo",
            overlap_source.join(".repo").to_str().expect("overlap repo"),
            "--include",
            "config.toml",
        ],
    );
    let overlap = run_fail(bin, &env, &["backup", "overlap"]);
    assert!(overlap.contains("service root and repo must not overlap"));

    let control_source = temp.path().join("control-source");
    write_file(&control_source, "bad\nname.toml", "source\n", 0o600);
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "control",
            "--root",
            control_source.to_str().expect("control root"),
            "--include",
            "**",
        ],
    );
    let control = run_fail(bin, &env, &["backup", "control"]);
    assert!(control.contains("path is not portable"));

    let hardlink_source = temp.path().join("hardlink-source");
    write_file(&hardlink_source, "config.toml", "source\n", 0o600);
    fs::hard_link(
        hardlink_source.join("config.toml"),
        hardlink_source.join("config.link"),
    )
    .expect("create hard link");
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "hardlink",
            "--root",
            hardlink_source.to_str().expect("hardlink root"),
            "--include",
            "config.toml",
        ],
    );
    let hardlink = run_fail(bin, &env, &["backup", "hardlink"]);
    assert!(hardlink.contains("metadata loss"));
    let allowed = run_ok(bin, &env, &["backup", "--allow-metadata-loss", "hardlink"]);
    assert!(allowed.contains("copied 1 files"));
    assert!(
        env.data
            .join("lattice/repos/hardlink/config.toml")
            .is_file()
    );
}

#[test]
fn tampered_manifest_and_destination_symlinks_do_not_escape_sandbox() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let tampered_root = temp.path().join("tampered-root");
    let tampered_repo = temp.path().join("tampered-repo");
    fs::create_dir_all(&tampered_root).expect("create tampered root");
    fs::create_dir_all(tampered_repo.join(".lattice")).expect("create tampered manifest dir");
    fs::write(
        env.config.join("lattice/services/tampered.toml"),
        format!(
            r#"
name = "tampered"
root = "{}"
repo = "{}"
include = ["config.toml"]
"#,
            tampered_root.display(),
            tampered_repo.display()
        ),
    )
    .expect("write tampered service config");

    fs::write(
        tampered_repo.join(".lattice/manifest.toml"),
        r#"version = 1
entries = [
  { path = "../escaped.txt", mode = "0600" },
]
"#,
    )
    .expect("write traversal manifest");
    let traversal = run_fail(bin, &env, &["restore", "--dry-run", "tampered"]);
    assert!(traversal.contains("unsafe relative path"));
    assert!(!temp.path().join("escaped.txt").exists());

    fs::write(
        tampered_repo.join(".lattice/manifest.toml"),
        format!(
            r#"version = 1
entries = [
  {{ path = "{}", mode = "0600" }},
]
"#,
            temp.path().join("absolute-escape.txt").display()
        ),
    )
    .expect("write absolute path manifest");
    let absolute = run_fail(bin, &env, &["restore", "--force", "tampered"]);
    assert!(absolute.contains("unsafe relative path"));
    assert!(!temp.path().join("absolute-escape.txt").exists());

    let destlink_source = temp.path().join("destlink-source");
    let destlink_repo = temp.path().join("destlink-repo");
    let outside_target = temp.path().join("outside-target.txt");
    write_file(&destlink_source, "config.toml", "source version\n", 0o600);
    fs::create_dir_all(&destlink_repo).expect("create destlink repo");
    fs::write(&outside_target, "outside stays put\n").expect("write outside target");
    symlink(&outside_target, destlink_repo.join("config.toml"))
        .expect("create repo destination symlink");
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "destlink",
            "--root",
            destlink_source.to_str().expect("destlink root"),
            "--repo",
            destlink_repo.to_str().expect("destlink repo"),
            "--include",
            "config.toml",
        ],
    );
    let destlink = run_fail(bin, &env, &["backup", "destlink"]);
    assert!(destlink.contains("backup destination is a symlink"));
    assert_eq!(
        fs::read_to_string(&outside_target).unwrap(),
        "outside stays put\n"
    );
    assert!(!destlink_repo.join(".lattice/manifest.toml").exists());

    let createdirs_root = temp.path().join("createdirs-root");
    let createdirs_repo = temp.path().join("createdirs-repo");
    let outside_dir = temp.path().join("outside-dir");
    fs::create_dir_all(&createdirs_root).expect("create createdirs root");
    fs::create_dir_all(createdirs_repo.join(".lattice")).expect("create createdirs manifest dir");
    fs::create_dir_all(&outside_dir).expect("create outside dir");
    symlink(&outside_dir, createdirs_root.join("cache")).expect("create restore dir symlink");
    fs::write(
        createdirs_repo.join(".lattice/manifest.toml"),
        "version = 1\nentries = []\n",
    )
    .expect("write empty manifest");
    fs::write(
        env.config.join("lattice/services/createdirs.toml"),
        format!(
            r#"
name = "createdirs"
root = "{}"
repo = "{}"
include = ["config.toml"]

[restore]
create_dirs = [
  {{ path = "cache/nested", mode = "0700" }},
]
"#,
            createdirs_root.display(),
            createdirs_repo.display()
        ),
    )
    .expect("write createdirs service config");
    let createdirs = run_fail(bin, &env, &["restore", "createdirs"]);
    assert!(createdirs.contains("destination parent is a symlink"));
    assert!(!outside_dir.join("nested").exists());
}

#[test]
fn adopt_failure_does_not_persist_tracking_or_copy_secret_like_files() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("adopt-source");
    write_file(&source, "safe.toml", "theme = \"dark\"\n", 0o600);
    write_file(
        &source,
        "secret.env",
        &format!(
            "OPENAI_API_KEY={}proj_fake_but_token_shaped\n",
            ["s", "k-"].concat()
        ),
        0o600,
    );
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "adoptedge",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "safe.toml",
        ],
    );

    let failed_adopt = run_fail(bin, &env, &["adopt", "adoptedge", "secret.env"]);
    assert!(failed_adopt.contains("secret-looking content"));

    let service = run_ok(bin, &env, &["service", "show", "adoptedge"]);
    assert!(!service.contains("secret.env"));
    let repo = env.data.join("lattice/repos/adoptedge");
    assert!(!repo.join("secret.env").exists());
}

#[test]
fn repo_failure_harness_covers_git_errors_and_secret_commit_guard() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("repo-source");
    write_file(&source, "settings.toml", "theme = \"dark\"\n", 0o600);
    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "repoedge",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "settings.toml",
        ],
    );
    run_ok(bin, &env, &["backup", "repoedge"]);

    let repo = env.data.join("lattice/repos/repoedge");
    let not_git = run_fail(bin, &env, &["repo", "status", "repoedge"]);
    assert!(not_git.contains("repo is not a git repository"));

    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "lattice@example.test"]);
    run_git(&repo, &["config", "user.name", "Lattice Test"]);

    let push_without_remote = run_fail(bin, &env, &["repo", "push", "repoedge"]);
    assert!(push_without_remote.contains("git exited"));

    fs::write(
        repo.join("leak.env"),
        format!(
            "OPENAI_API_KEY={}proj_fake_but_token_shaped\n",
            ["s", "k-"].concat()
        ),
    )
    .expect("write repo secret");
    let blocked_commit = run_fail(
        bin,
        &env,
        &["repo", "commit", "repoedge", "--message", "backup configs"],
    );
    assert!(blocked_commit.contains("secret-looking content"));
    assert!(git_log_is_empty(&repo));
}

#[test]
fn diff_harness_hides_binary_or_unreadable_content() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("binary-source");
    let repo = temp.path().join("binary-repo");
    fs::create_dir_all(&source).expect("create source");
    fs::write(source.join("blob.bin"), [0, 159, 146, 150]).expect("write binary source");
    fs::write(
        env.config.join("lattice/services/binary.toml"),
        format!(
            r#"
name = "binary"
root = "{}"
repo = "{}"
include = ["blob.bin"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write binary service");

    run_ok(bin, &env, &["backup", "binary"]);
    fs::write(source.join("blob.bin"), [0, 1, 2, 3, 255]).expect("modify binary source");

    let diff = run_ok(bin, &env, &["diff", "binary"]);
    assert!(diff.contains("diff blob.bin"));
    assert!(diff.contains("binary content differs; line diff hidden"));
    assert!(!diff.contains("OPENAI_API_KEY"));
}

#[test]
fn machine_readable_commands_honor_only_and_exclude_selectors() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("selector-source");
    let repo = temp.path().join("selector-repo");
    write_file(&source, "config.toml", "model = \"gpt-5.5\"\n", 0o600);
    write_file(
        &source,
        "agents/reviewer.toml",
        "name = \"reviewer\"\n",
        0o600,
    );
    write_file(&source, "notes.md", "local notes\n", 0o600);
    fs::create_dir_all(source.join("profiles/empty")).expect("create empty profile");
    fs::write(
        env.config.join("lattice/services/selector.toml"),
        format!(
            r#"
name = "selector"
root = "{}"
repo = "{}"
include = ["**"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write selector service config");

    let status = run_json(
        bin,
        &env,
        &[
            "status",
            "--json",
            "--only",
            "config.toml",
            "--exclude",
            "notes.md",
            "selector",
        ],
    );
    assert_eq!(status["service"], "selector");
    assert_eq!(status["included_files"], 1);
    assert_eq!(status["files"], serde_json::json!(["config.toml"]));

    let dry_backup = run_json(
        bin,
        &env,
        &[
            "backup",
            "--dry-run",
            "--json",
            "--only",
            "config.toml",
            "--exclude",
            "notes.md",
            "selector",
        ],
    );
    assert_eq!(dry_backup["dry_run"], true);
    assert_eq!(dry_backup["would_copy"], 1);
    assert_eq!(dry_backup["files"], serde_json::json!(["config.toml"]));

    let backup = run_json(
        bin,
        &env,
        &[
            "backup",
            "--json",
            "--only",
            "config.toml",
            "--exclude",
            "notes.md",
            "selector",
        ],
    );
    assert_eq!(backup["copied"], 1);
    assert!(repo.join("config.toml").exists());
    assert!(!repo.join("agents/reviewer.toml").exists());
    assert!(!repo.join("notes.md").exists());

    fs::write(source.join("config.toml"), "local drift\n").expect("create selected drift");
    fs::write(source.join("agents/reviewer.toml"), "ignored drift\n")
        .expect("create unselected drift");

    let diff = run_json(
        bin,
        &env,
        &[
            "diff",
            "--json",
            "--only",
            "config.toml",
            "--exclude",
            "agents/**",
            "selector",
        ],
    );
    assert_eq!(diff["diffs"][0]["path"], "config.toml");
    assert_eq!(diff["diffs"].as_array().expect("diff array").len(), 1);

    let dry_restore = run_json(
        bin,
        &env,
        &[
            "restore",
            "--dry-run",
            "--json",
            "--only",
            "config.toml",
            "--exclude",
            "agents/**",
            "selector",
        ],
    );
    assert_eq!(dry_restore["dry_run"], true);
    assert_eq!(dry_restore["would_restore"], 1);
    assert_eq!(dry_restore["entries"], serde_json::json!(["config.toml"]));
    assert_eq!(dry_restore["conflicts"], serde_json::json!(["config.toml"]));
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

#[test]
fn snapshot_history_supports_dry_run_restore_and_prune() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("snapshot-source");
    let repo = temp.path().join("snapshot-repo");
    write_file(&source, "config.toml", "repo version\n", 0o600);
    fs::write(
        env.config.join("lattice/services/snap.toml"),
        format!(
            r#"
name = "snap"
root = "{}"
repo = "{}"
include = ["config.toml"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write snap config");
    run_ok(bin, &env, &["backup", "snap"]);
    fs::write(source.join("config.toml"), "local version\n").expect("write local version");
    let restore = run_ok(bin, &env, &["restore", "--force", "snap"]);
    assert!(restore.contains("snapshot:"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).unwrap(),
        "repo version\n"
    );

    let snapshots = run_json(bin, &env, &["snapshot", "list", "--json"]);
    let snapshot_id = snapshots["snapshots"][0]["id"]
        .as_str()
        .expect("snapshot id")
        .to_string();
    assert_eq!(snapshots["snapshots"][0]["service"], "snap");
    assert_eq!(snapshots["snapshots"][0]["files"], 1);

    let show = run_json(bin, &env, &["snapshot", "show", "--json", &snapshot_id]);
    assert_eq!(show["id"], snapshot_id);
    assert_eq!(show["entries"], serde_json::json!(["config.toml"]));

    let undo_plan = run_json(bin, &env, &["undo", "--dry-run", "--json", &snapshot_id]);
    assert_eq!(undo_plan["dry_run"], true);
    assert_eq!(undo_plan["would_restore"], 1);
    assert_eq!(undo_plan["entries"], serde_json::json!(["config.toml"]));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).unwrap(),
        "repo version\n"
    );

    let outside = temp.path().join("outside-target");
    fs::write(&outside, "outside\n").expect("write outside target");
    fs::remove_file(source.join("config.toml")).expect("remove config before symlink");
    symlink(&outside, source.join("config.toml")).expect("symlink config to outside target");
    let symlink_error = run_fail(bin, &env, &["undo", "--yes", "--json", &snapshot_id]);
    assert!(symlink_error.contains("symlink"));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside\n");
    fs::remove_file(source.join("config.toml")).expect("remove symlink");
    fs::write(source.join("config.toml"), "repo version\n")
        .expect("restore repo version before undo");

    let undo = run_json(bin, &env, &["undo", "--yes", "--json", &snapshot_id]);
    assert_eq!(undo["dry_run"], false);
    assert_eq!(undo["restored"], 1);
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).unwrap(),
        "local version\n"
    );

    let prune_plan = run_json(
        bin,
        &env,
        &["snapshot", "prune", "--dry-run", "--json", "--keep", "0"],
    );
    assert_eq!(prune_plan["dry_run"], true);
    assert_eq!(prune_plan["would_remove"], 1);
    assert!(env.state.join("lattice/snapshots").exists());

    let pruned = run_json(
        bin,
        &env,
        &["snapshot", "prune", "--yes", "--json", "--keep", "0"],
    );
    assert_eq!(pruned["dry_run"], false);
    assert_eq!(pruned["removed"], 1);
    let snapshots_after_prune = run_json(bin, &env, &["snapshot", "list", "--json"]);
    assert!(
        snapshots_after_prune["snapshots"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn discover_suggests_conservative_generic_services_with_json() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    fs::create_dir_all(env.home.join(".config/tool/cache")).expect("create cache");
    fs::create_dir_all(env.home.join(".config/tool/sessions")).expect("create sessions");
    fs::write(
        env.home.join(".config/tool/settings.toml"),
        "theme = 'dark'\n",
    )
    .expect("write settings");
    fs::write(env.home.join(".config/tool/cache/state.db"), "cache\n").expect("write cache");
    fs::write(env.home.join(".config/tool/token.json"), "token\n").expect("write token");
    symlink(
        env.home.join(".config/tool"),
        env.home.join(".config/linked-tool"),
    )
    .expect("symlink config dir");
    symlink(
        env.home.join(".config/tool/settings.toml"),
        env.home.join(".bashrc"),
    )
    .expect("symlink shell rc");
    fs::write(env.home.join(".zshrc"), "export EDITOR=vim\n").expect("write zshrc");

    let discovery = run_json(bin, &env, &["discover", "--json"]);
    let services = discovery["suggestions"].as_array().expect("suggestions");
    assert!(services.iter().any(|item| item["name"] == "tool"));
    assert!(services.iter().all(|item| item["name"] != "linked-tool"));
    assert!(services.iter().any(|item| item["name"] == "shell"));
    let tool = services
        .iter()
        .find(|item| item["name"] == "tool")
        .expect("tool suggestion");
    assert_eq!(
        tool["root"],
        env.home.join(".config/tool").display().to_string()
    );
    assert!(
        tool["include"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("settings.toml"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("cache/**"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("sessions/**"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("token.json"))
    );
    assert_eq!(discovery["mutated"], false);
    assert!(!env.config.join("lattice/services/tool.toml").exists());
}

struct TestEnv {
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
}

impl TestEnv {
    fn new(root: &Path) -> Self {
        Self {
            home: root.join("home"),
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
        }
    }
}

fn run_ok(bin: &str, env: &TestEnv, args: &[&str]) -> String {
    let output = Command::new(bin)
        .args(args)
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.config)
        .env("XDG_DATA_HOME", &env.data)
        .env("XDG_STATE_HOME", &env.state)
        .env("XDG_CACHE_HOME", &env.cache)
        .output()
        .expect("run command");

    assert!(
        output.status.success(),
        "command {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_fail(bin: &str, env: &TestEnv, args: &[&str]) -> String {
    let output = Command::new(bin)
        .args(args)
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.config)
        .env("XDG_DATA_HOME", &env.data)
        .env("XDG_STATE_HOME", &env.state)
        .env("XDG_CACHE_HOME", &env.cache)
        .output()
        .expect("run command");

    assert!(
        !output.status.success(),
        "command {:?} unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn run_json(bin: &str, env: &TestEnv, args: &[&str]) -> serde_json::Value {
    let output = run_ok(bin, env, args);
    serde_json::from_str(&output).unwrap_or_else(|error| {
        panic!("command {args:?} did not emit valid json: {error}\noutput:\n{output}")
    })
}

fn write_file(root: &Path, relative: &str, body: &str, mode: u32) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(&path, body).expect("write file");
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set permissions");
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_log_is_empty(repo: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "--oneline"])
        .output()
        .expect("run git log");
    !output.status.success() || output.stdout.is_empty()
}
