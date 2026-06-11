use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, mode, run_fail, run_ok, write_file};

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
