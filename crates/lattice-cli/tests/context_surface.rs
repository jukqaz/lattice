use std::fs;

use tempfile::tempdir;

mod support;

use support::{TestEnv, run_fail, run_json, run_ok, write_file};

#[test]
fn context_labels_gate_services_and_explain_inactive_json() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);

    let shared_source = temp.path().join("shared-source");
    let work_source = temp.path().join("work-source");
    let home_source = temp.path().join("home-source");
    write_file(&shared_source, "config.toml", "shared = true\n", 0o600);
    write_file(&work_source, "config.toml", "work = true\n", 0o600);
    write_file(&home_source, "config.toml", "home = true\n", 0o600);

    fs::write(
        env.config.join("lattice/lattice.toml"),
        r#"version = 1
profile = "main"
contexts = ["laptop", "work"]

[[groups]]
name = "daily"
description = "Shared plus current context services"
services = ["shared", "work", "home"]
"#,
    )
    .expect("write global config with active contexts");

    fs::write(
        env.config.join("lattice/services/shared.toml"),
        format!(
            r#"name = "shared"
root = "{}"
include = ["config.toml"]
"#,
            shared_source.display()
        ),
    )
    .expect("write shared service");
    fs::write(
        env.config.join("lattice/services/work.toml"),
        format!(
            r#"name = "work"
root = "{}"
include = ["config.toml"]

[conditions]
contexts = ["work"]
"#,
            work_source.display()
        ),
    )
    .expect("write work service");
    fs::write(
        env.config.join("lattice/services/home.toml"),
        format!(
            r#"name = "home"
root = "{}"
include = ["config.toml"]

[conditions]
contexts = ["home"]
"#,
            home_source.display()
        ),
    )
    .expect("write home service");

    let context = run_json(bin, &env, &["context", "show", "--json"]);
    assert_eq!(context["profile"], "main");
    assert_eq!(context["contexts"], serde_json::json!(["laptop", "work"]));
    assert_eq!(context["os"], std::env::consts::OS);
    assert!(context["hostname"].is_string() || context["hostname"].is_null());

    let home_status = run_json(bin, &env, &["status", "--json", "home"]);
    assert_eq!(home_status["active"], false);
    assert_eq!(home_status["inactive_reasons"][0]["kind"], "contexts");
    assert_eq!(
        home_status["inactive_reasons"][0]["missing"],
        serde_json::json!(["home"])
    );
    assert_eq!(
        home_status["inactive_reasons"][0]["actual"],
        serde_json::json!(["laptop", "work"])
    );

    let home_plan = run_json(bin, &env, &["plan", "--json", "home"]);
    assert_eq!(home_plan["ready"], false);
    assert_eq!(home_plan["active"], false);
    assert_eq!(home_plan["inactive_reasons"][0]["kind"], "contexts");
    assert_eq!(home_plan["backup_would_copy"], 0);

    let blocked_backup = run_fail(bin, &env, &["backup", "home"]);
    assert!(blocked_backup.contains("inactive in current context"));
    assert!(blocked_backup.contains("missing contexts: home"));

    let group_status = run_json(bin, &env, &["group", "status", "--json", "daily"]);
    assert_eq!(group_status["active_services"], 2);
    assert_eq!(group_status["included_files"], 2);
    let home_summary = group_status["services"]
        .as_array()
        .expect("service summaries")
        .iter()
        .find(|summary| summary["service"] == "home")
        .expect("home service summary");
    assert_eq!(home_summary["active"], false);
    assert_eq!(home_summary["inactive_reasons"][0]["kind"], "contexts");

    let group_plan = run_json(bin, &env, &["group", "plan", "--json", "daily"]);
    assert_eq!(group_plan["active_services"], 2);
    let inactive_home_plan = group_plan["services"]
        .as_array()
        .expect("service plans")
        .iter()
        .find(|summary| summary["service"] == "home")
        .expect("home service plan");
    assert_eq!(inactive_home_plan["backup_would_copy"], 0);
    assert_eq!(
        inactive_home_plan["inactive_reasons"][0]["missing"],
        serde_json::json!(["home"])
    );
}

#[test]
fn service_add_accepts_context_conditions_without_reordering_existing_labels() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    let source = temp.path().join("work-source");
    write_file(&source, "config.toml", "work = true\n", 0o600);

    run_ok(
        bin,
        &env,
        &[
            "service",
            "add",
            "work",
            "--root",
            source.to_str().expect("source path"),
            "--include",
            "config.toml",
            "--context",
            "work",
            "--context",
            "laptop",
        ],
    );

    let show = run_ok(bin, &env, &["service", "show", "work"]);
    assert!(show.contains("[conditions]"));
    assert!(show.contains("contexts = ["));
    assert!(show.contains("\"work\""));
    assert!(show.contains("\"laptop\""));
}
