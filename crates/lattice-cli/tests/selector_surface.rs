use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_json, run_ok, write_file};

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
