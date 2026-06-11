use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, git_log_is_empty, run_fail, run_git, run_ok, write_file};

#[test]
fn adopt_repo_and_diff_happy_paths_cover_tracked_service() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let source = temp.path().join("adopt-repo-diff-source");
    write_file(&source, ".zshrc", "export EDITOR=vim\n", 0o644);
    write_file(&source, ".zprofile", "path+=('/opt/homebrew/bin')\n", 0o644);

    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "shell",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            ".zshrc",
            "--include",
            ".zprofile",
        ],
    );

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
