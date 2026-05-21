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

    let version = run_ok(bin, &env, &["--version"]);
    assert!(version.contains("lattice"));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));

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
fn mvp2_commands_cover_presets_repo_secrets_track_adopt_diff_and_tui() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("home");
    write_file(&source, ".zshrc", "export EDITOR=vim\n", 0o644);
    write_file(&source, ".zprofile", "path+=('/opt/homebrew/bin')\n", 0o644);

    let presets = run_ok(bin, &env, &["preset", "list"]);
    assert!(presets.contains("codex"));
    assert!(presets.contains("zsh"));
    assert!(presets.contains("ssh"));

    let zsh_preset = run_ok(bin, &env, &["preset", "show", "zsh"]);
    assert!(zsh_preset.contains(".zshrc"));
    assert!(zsh_preset.contains(".zsh_history"));

    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "shell",
            "--root",
            source.to_str().expect("source path"),
            "--preset",
            "zsh",
        ],
    );

    let status = run_ok(bin, &env, &["status", "shell"]);
    assert!(status.contains("included files: 2"));

    run_ok(bin, &env, &["track", "shell", ".config/starship.toml"]);
    let tracked = run_ok(bin, &env, &["service", "show", "shell"]);
    assert!(tracked.contains(".config/starship.toml"));

    run_ok(
        bin,
        &env,
        &[
            "secret",
            "add",
            "shell",
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
    let secrets = run_ok(bin, &env, &["secret", "list", "shell"]);
    assert!(secrets.contains("github-token backend=rbw item=GitHub token"));
    assert!(!secrets.contains("password="));
    let secret_check = run_ok(bin, &env, &["secret", "check", "shell"]);
    assert!(secret_check.contains("value=not-read"));

    let repo = env.data.join("lattice/repos/shell");
    run_ok(bin, &env, &["backup", "shell"]);
    run_git(&repo, &["init"]);
    run_git(&repo, &["config", "user.email", "lattice@example.test"]);
    run_git(&repo, &["config", "user.name", "Lattice Test"]);

    let repo_status = run_ok(bin, &env, &["repo", "status", "shell"]);
    assert!(repo_status.contains("##"));
    run_ok(
        bin,
        &env,
        &["repo", "commit", "shell", "--message", "initial backup"],
    );

    fs::write(source.join(".zshrc"), "export EDITOR=nvim\n").expect("modify zshrc");
    let diff = run_ok(bin, &env, &["diff", "shell"]);
    assert!(diff.contains("diff .zshrc"));
    assert!(diff.contains("+export EDITOR=nvim"));

    let adopt = run_ok(bin, &env, &["adopt", "shell", ".zprofile"]);
    assert!(adopt.contains("copied"));

    let tui = run_ok(bin, &env, &["tui", "--dry-run"]);
    assert!(tui.contains("service list"));
    assert!(tui.contains("backup dry-run"));
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
    assert_eq!(mode(&env.config.join("lattice/services/codex.toml")), 0o600);

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

    let unknown_preset = run_fail(
        bin,
        &env,
        &[
            "service",
            "add",
            "badpreset",
            "--root",
            source.to_str().expect("source path"),
            "--preset",
            "__missing__",
        ],
    );
    assert!(unknown_preset.contains("unknown preset"));

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
