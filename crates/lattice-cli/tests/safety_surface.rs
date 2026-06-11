use std::fs;
use std::os::unix::fs::symlink;

use tempfile::tempdir;

mod support;

use support::{TestEnv, mode, run_fail, run_ok, write_file};

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
