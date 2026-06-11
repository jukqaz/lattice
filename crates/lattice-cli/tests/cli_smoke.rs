use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

use tempfile::tempdir;

mod support;

use support::{TestEnv, git_log_is_empty, mode, run_fail, run_git, run_json, run_ok, write_file};

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

    fs::write(
        tampered_repo.join(".lattice/manifest.toml"),
        r#"version = 999
entries = [
  { path = "config.toml", mode = "0600" },
]
"#,
    )
    .expect("write unsupported manifest version");
    let unsupported = run_fail(bin, &env, &["restore", "--dry-run", "tampered"]);
    assert!(unsupported.contains("unsupported manifest version: 999"));
    assert!(!tampered_root.join("config.toml").exists());

    for (manifest_body, expected_error, outside_probe) in [
        (
            r#"version = 1
entries = [
  { path = "", mode = "0600" },
]
"#,
            "unsafe relative path",
            tampered_root.join("empty-path-probe"),
        ),
        (
            r#"version = 1
entries = [
  { path = "bad\u0007path.toml", mode = "0600" },
]
"#,
            "path is not portable because it contains control characters",
            tampered_root.join("bad\u{0007}path.toml"),
        ),
    ] {
        fs::write(tampered_repo.join(".lattice/manifest.toml"), manifest_body)
            .expect("write negative manifest fixture");
        let rejected = run_fail(bin, &env, &["restore", "--dry-run", "tampered"]);
        assert!(
            rejected.contains(expected_error),
            "expected {expected_error:?}, got:\n{rejected}"
        );
        assert!(!outside_probe.exists());
    }

    fs::write(tampered_repo.join("Config.toml"), "upper\n").expect("write upper config");
    fs::write(tampered_repo.join("config.toml"), "lower\n").expect("write lower config");
    fs::write(
        tampered_repo.join(".lattice/manifest.toml"),
        r#"version = 1
entries = [
  { path = "Config.toml", mode = "0600" },
  { path = "config.toml", mode = "0600" },
]
"#,
    )
    .expect("write colliding manifest paths");
    let collision = run_fail(bin, &env, &["restore", "--force", "tampered"]);
    assert!(collision.contains("portable path collision"));
    assert!(!tampered_root.join("Config.toml").exists());
    assert!(!tampered_root.join("config.toml").exists());

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
