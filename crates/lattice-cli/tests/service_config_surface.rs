use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_fail, run_ok, write_file};

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
