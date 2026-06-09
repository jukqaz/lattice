use std::fs;
use std::os::unix::fs::symlink;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_fail, run_json, run_ok, write_file};

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
fn snapshot_failure_paths_do_not_traverse_or_delete_symlinked_roots() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let outside_snapshots = temp.path().join("outside-snapshots");
    fs::create_dir_all(outside_snapshots.join("123/snap"))
        .expect("create outside snapshot fixture");
    fs::write(
        outside_snapshots.join("123/snap/config.toml"),
        "outside snapshot\n",
    )
    .expect("write outside snapshot file");

    let snapshots_root = env.state.join("lattice/snapshots");
    fs::create_dir_all(snapshots_root.parent().expect("snapshots parent"))
        .expect("create snapshots parent");
    symlink(&outside_snapshots, &snapshots_root).expect("symlink snapshots root");

    let list_error = run_fail(bin, &env, &["snapshot", "list", "--json"]);
    assert!(list_error.contains("snapshot root is not a directory"));
    let prune_error = run_fail(
        bin,
        &env,
        &["snapshot", "prune", "--yes", "--json", "--keep", "0"],
    );
    assert!(prune_error.contains("snapshot root is not a directory"));
    assert!(outside_snapshots.join("123/snap/config.toml").exists());

    let traversal = run_fail(bin, &env, &["snapshot", "show", "--json", "../outside"]);
    assert!(traversal.contains("snapshot root is not a directory"));
}

#[test]
fn snapshot_undo_preflights_failure_paths_without_partial_restore() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("snapshot-preflight-source");
    let repo = temp.path().join("snapshot-preflight-repo");
    write_file(&source, "a.toml", "repo a\n", 0o600);
    write_file(&source, "nested/b.toml", "repo b\n", 0o600);
    fs::write(
        env.config.join("lattice/services/preflight.toml"),
        format!(
            r#"
name = "preflight"
root = "{}"
repo = "{}"
include = ["a.toml", "nested/b.toml"]
"#,
            source.display(),
            repo.display()
        ),
    )
    .expect("write preflight config");
    run_ok(bin, &env, &["backup", "preflight"]);

    fs::write(source.join("a.toml"), "local a\n").expect("write local a");
    fs::write(source.join("nested/b.toml"), "local b\n").expect("write local b");
    run_ok(bin, &env, &["restore", "--force", "preflight"]);

    let snapshots = run_json(bin, &env, &["snapshot", "list", "--json"]);
    let snapshot_id = snapshots["snapshots"][0]["id"]
        .as_str()
        .expect("snapshot id")
        .to_string();

    fs::remove_file(source.join("nested/b.toml")).expect("remove restored nested file");
    fs::remove_dir(source.join("nested")).expect("remove nested directory");
    fs::write(source.join("nested"), "blocking parent\n").expect("write parent collision");

    let dry_run_error = run_fail(bin, &env, &["undo", "--dry-run", "--json", &snapshot_id]);
    assert!(
        dry_run_error.contains("snapshot restore parent is not a directory"),
        "unexpected undo dry-run error:\n{dry_run_error}"
    );
    assert_eq!(
        fs::read_to_string(source.join("a.toml")).expect("a untouched after dry-run"),
        "repo a\n"
    );
    assert_eq!(
        fs::read_to_string(source.join("nested"))
            .expect("parent collision untouched after dry-run"),
        "blocking parent\n"
    );

    let error = run_fail(bin, &env, &["undo", "--yes", "--json", &snapshot_id]);
    assert!(
        error.contains("snapshot restore parent is not a directory"),
        "unexpected undo error:\n{error}"
    );
    assert_eq!(
        fs::read_to_string(source.join("a.toml")).expect("a untouched"),
        "repo a\n"
    );
    assert_eq!(
        fs::read_to_string(source.join("nested")).expect("parent collision untouched"),
        "blocking parent\n"
    );
}
