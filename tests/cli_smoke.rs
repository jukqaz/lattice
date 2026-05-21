use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

#[test]
fn init_doctor_list_backup_and_restore_codex_service() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    assert!(env.config.join("lattice/lattice.toml").exists());
    assert!(env.config.join("lattice/services/codex.toml").exists());

    let doctor = run_ok(bin, &env, &["doctor"]);
    assert!(doctor.contains("config:"));
    assert!(doctor.contains("rbw:"));
    assert!(doctor.contains("bw:"));

    let validate = run_ok(bin, &env, &["validate"]);
    assert!(validate.contains("valid config"));
    assert!(validate.contains("services: 1"));

    let source = temp.path().join("codex-source");
    let repo = temp.path().join("codex-repo");
    let hook_marker = temp.path().join("after-restore-hook.txt");
    let confirm_marker = temp.path().join("confirm-hook.txt");
    write_file(&source, "config.toml", "model = \"gpt-5.5\"\n", 0o600);
    write_file(&source, "bin/mcp-rbw", "#!/usr/bin/env bash\n", 0o700);
    write_file(&source, "auth.json", "{}\n", 0o600);
    fs::write(
        env.config.join("lattice/services/codex.toml"),
        format!(
            r#"
name = "codex"
root = "{}"
repo = "{}"
preset = "codex"

[restore]
create_dirs = [
  {{ path = "shell_snapshots", mode = "0700" }},
]

[[permissions]]
path = "config.toml"
mode = "0600"

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
    assert!(services.contains("codex"));

    let status = run_ok(bin, &env, &["status", "codex"]);
    assert!(status.contains("service: codex"));
    assert!(status.contains("included files: 2"));
    assert!(status.contains("manifest: missing"));

    let dry_backup = run_ok(bin, &env, &["backup", "--dry-run", "codex"]);
    assert!(dry_backup.contains("would copy 2 files"));
    assert!(dry_backup.contains("would run hook before_backup: confirm required"));
    assert!(!repo.join("config.toml").exists());
    assert!(!repo.join(".lattice/manifest.toml").exists());
    assert!(!confirm_marker.exists());

    let backup = run_ok(bin, &env, &["backup", "codex"]);
    assert!(backup.contains("copied 2 files"));
    assert!(backup.contains("skipped hook before_backup: confirm required"));
    assert!(repo.join("config.toml").exists());
    assert!(repo.join("bin/mcp-rbw").exists());
    assert!(!repo.join("auth.json").exists());
    assert!(repo.join(".lattice/manifest.toml").exists());

    fs::write(source.join("config.toml"), "local drift\n").expect("create local drift");
    let dry_restore = run_ok(bin, &env, &["restore", "--dry-run", "codex"]);
    assert!(dry_restore.contains("would restore 2 files"));
    assert!(dry_restore.contains("conflicts: 1"));
    assert!(dry_restore.contains("conflict config.toml"));
    assert!(dry_restore.contains("would run hook after_restore: write after restore marker"));
    assert!(!hook_marker.exists());

    let failed_restore = run_fail(bin, &env, &["restore", "codex"]);
    assert!(failed_restore.contains("restore conflicts"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("local drift"),
        "local drift\n"
    );

    let forced_restore = run_ok(bin, &env, &["restore", "--force", "codex"]);
    assert!(forced_restore.contains("restored 2 files"));
    assert!(forced_restore.contains("snapshot:"));
    assert!(forced_restore.contains("ran hook after_restore: write after restore marker"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("forced restore"),
        "model = \"gpt-5.5\"\n"
    );
    assert_eq!(
        fs::read_to_string(&hook_marker).expect("hook marker"),
        "after_restore"
    );

    let backup_yes = run_ok(bin, &env, &["backup", "--yes", "codex"]);
    assert!(backup_yes.contains("ran hook before_backup: confirm required"));
    assert_eq!(
        fs::read_to_string(&confirm_marker).expect("confirm marker"),
        "confirmed"
    );

    fs::remove_dir_all(&source).expect("remove source");
    let restore = run_ok(bin, &env, &["restore", "codex"]);
    assert!(restore.contains("restored 2 files"));
    assert_eq!(
        fs::read_to_string(source.join("config.toml")).expect("restored config"),
        "model = \"gpt-5.5\"\n"
    );
    assert_eq!(mode(&source.join("config.toml")), 0o600);
    assert_eq!(mode(&source.join("bin/mcp-rbw")), 0o700);
    assert!(source.join("shell_snapshots").is_dir());
    assert_eq!(mode(&source.join("shell_snapshots")), 0o700);
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

fn write_file(root: &Path, relative: &str, body: &str, mode: u32) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(&path, body).expect("write file");
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set permissions");
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}
