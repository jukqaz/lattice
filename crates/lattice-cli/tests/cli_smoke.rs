use std::fs;
use std::process::Command;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_ok, write_file};

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

    run_ok(
        bin,
        &env,
        &[
            "secret",
            "add",
            "zsh",
            "api-key",
            "--backend",
            "env",
            "--env",
            "LATTICE_TEST_API_KEY",
        ],
    );
    let env_secrets = run_ok(bin, &env, &["secret", "list", "zsh"]);
    assert!(env_secrets.contains(
        "api-key backend=env item=LATTICE_TEST_API_KEY field=- env=LATTICE_TEST_API_KEY"
    ));
    assert!(!env_secrets.contains("test-value"));
    let env_secret_check = Command::new(bin)
        .args(["secret", "check", "zsh"])
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.config)
        .env("XDG_DATA_HOME", &env.data)
        .env("XDG_STATE_HOME", &env.state)
        .env("XDG_CACHE_HOME", &env.cache)
        .env("LATTICE_TEST_API_KEY", "test-value")
        .output()
        .expect("run env secret check");
    assert!(env_secret_check.status.success());
    let env_secret_check = String::from_utf8_lossy(&env_secret_check.stdout);
    assert!(env_secret_check.contains(
        "api-key backend=env status=set item=LATTICE_TEST_API_KEY env=LATTICE_TEST_API_KEY value=not-read"
    ));
    assert!(!env_secret_check.contains("test-value"));

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
