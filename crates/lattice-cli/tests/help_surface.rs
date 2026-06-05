use tempfile::tempdir;

mod support;

use support::{TestEnv, run_ok};

#[test]
fn cli_help_surfaces_actionable_descriptions() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    let help = run_ok(bin, &env, &["--help"]);
    assert!(help.contains("A small dotfiles and configuration manager"));
    assert!(help.contains("Manage app catalog shortcuts"));
    assert!(help.contains("Check new-machine readiness without mutating state"));
    assert!(help.contains("Summarize backup and restore risk before changing files"));
    assert!(help.contains("Inspect and prune restore safety snapshots"));
    assert!(help.contains("Suggest local service candidates without mutating state"));
    assert!(help.contains("Inspect service groups without mutating state"));

    let app_help = run_ok(bin, &env, &["app", "--help"]);
    assert!(app_help.contains("List built-in app catalog entries"));
    assert!(app_help.contains("Show the suggested config for an app"));
    assert!(app_help.contains("Create a service config from an app catalog entry"));

    let group_help = run_ok(bin, &env, &["group", "--help"]);
    assert!(group_help.contains("List configured service groups"));
    assert!(group_help.contains("Show one service group"));
    assert!(group_help.contains("Show grouped service status"));
    assert!(group_help.contains("Summarize grouped service plans"));

    let snapshot_help = run_ok(bin, &env, &["snapshot", "--help"]);
    assert!(snapshot_help.contains("List recorded restore safety snapshots"));
    assert!(snapshot_help.contains("Show files captured in one safety snapshot"));
    assert!(snapshot_help.contains("Delete old safety snapshots after keeping recent entries"));
}
