use std::fs;
use std::os::unix::fs::symlink;

use tempfile::tempdir;

mod support;

use support::{TestEnv, assert_json_keys, run_json, run_ok};

#[test]
fn discover_suggests_conservative_generic_services_with_json() {
    let temp = tempdir().expect("tempdir");
    let env = TestEnv::new(temp.path());
    let bin = env!("CARGO_BIN_EXE_lattice");

    run_ok(bin, &env, &["init", "--force"]);
    fs::create_dir_all(env.home.join(".config/tool/cache")).expect("create cache");
    fs::create_dir_all(env.home.join(".config/tool/sessions")).expect("create sessions");
    fs::write(
        env.home.join(".config/tool/settings.toml"),
        "theme = 'dark'\n",
    )
    .expect("write settings");
    fs::write(env.home.join(".config/tool/cache/state.db"), "cache\n").expect("write cache");
    fs::write(env.home.join(".config/tool/token.json"), "token\n").expect("write token");
    let openai_marker = ["s", "k-"].concat();
    fs::write(
        env.home.join(".config/tool/plain-key.toml"),
        format!("api_key = '{openai_marker}fake_discovery_token'\n"),
    )
    .expect("write secret-looking config");
    fs::create_dir_all(env.home.join(".config/onlysecret")).expect("create warning-only dir");
    fs::write(
        env.home.join(".config/onlysecret/config.toml"),
        format!("openai = '{openai_marker}fake_warning_only_token'\n"),
    )
    .expect("write warning-only secret-looking config");
    fs::create_dir_all(env.home.join(".config/mise")).expect("create app-named config dir");
    fs::write(env.home.join(".config/mise/config.toml"), "jobs = 4\n")
        .expect("write app-named config");
    fs::create_dir_all(env.home.join(".config/git")).expect("create warning-only app dir");
    fs::write(
        env.home.join(".config/git/config"),
        format!("token = '{openai_marker}fake_app_warning_only_token'\n"),
    )
    .expect("write warning-only app config");
    let github_marker = ["g", "hp_"].concat();
    fs::write(
        env.home.join(".profile"),
        format!("export TOKEN={github_marker}fake_discovery_token\n"),
    )
    .expect("write secret-looking profile");
    symlink(
        env.home.join(".config/tool"),
        env.home.join(".config/linked-tool"),
    )
    .expect("symlink config dir");
    symlink(
        env.home.join(".config/tool/settings.toml"),
        env.home.join(".bashrc"),
    )
    .expect("symlink shell rc");
    fs::write(env.home.join(".zshrc"), "export EDITOR=vim\n").expect("write zshrc");

    let discovery = run_json(bin, &env, &["discover", "--json"]);
    let services = discovery["suggestions"].as_array().expect("suggestions");
    assert!(services.iter().any(|item| item["name"] == "tool"));
    assert!(services.iter().any(|item| item["name"] == "onlysecret"));
    assert!(services.iter().all(|item| item["name"] != "linked-tool"));
    assert!(services.iter().any(|item| item["name"] == "shell"));
    let tool = services
        .iter()
        .find(|item| item["name"] == "tool")
        .expect("tool suggestion");
    assert_eq!(
        tool["root"],
        env.home.join(".config/tool").display().to_string()
    );
    assert!(
        tool["include"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("settings.toml"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("cache/**"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("sessions/**"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("token.json"))
    );
    assert!(
        tool["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("plain-key.toml"))
    );
    assert!(
        !tool["include"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("plain-key.toml"))
    );
    assert!(tool["warnings"].as_array().unwrap().iter().any(|warning| {
        let warning = warning.as_str().unwrap();
        warning.contains("plain-key.toml") && warning.contains("secret-looking content")
    }));
    let warning_only = services
        .iter()
        .find(|item| item["name"] == "onlysecret")
        .expect("warning-only suggestion");
    assert!(warning_only["include"].as_array().unwrap().is_empty());
    assert!(
        warning_only["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("config.toml"))
    );
    assert!(
        warning_only["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| {
                let warning = warning.as_str().unwrap();
                warning.contains("config.toml") && warning.contains("secret-looking content")
            })
    );
    let mise = services
        .iter()
        .find(|item| item["name"] == "mise")
        .expect("app-named config-dir suggestion");
    assert_eq!(mise["uses_app_catalog"], true);
    assert_eq!(
        mise["next_command"],
        format!(
            "lattice service add mise --root {} --include config.toml",
            env.home.join(".config/mise").display()
        )
    );
    let app_warning_only = services
        .iter()
        .find(|item| item["name"] == "git")
        .expect("warning-only app-named config-dir suggestion");
    assert_eq!(app_warning_only["uses_app_catalog"], true);
    assert!(app_warning_only["include"].as_array().unwrap().is_empty());
    assert_eq!(
        app_warning_only["next_command"],
        "review warnings before adding a service"
    );
    let shell = services
        .iter()
        .find(|item| item["name"] == "shell")
        .expect("shell suggestion");
    assert!(
        shell["include"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(".zshrc"))
    );
    assert!(
        !shell["include"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(".profile"))
    );
    assert!(
        shell["exclude"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(".profile"))
    );
    assert!(shell["warnings"].as_array().unwrap().iter().any(|warning| {
        let warning = warning.as_str().unwrap();
        warning.contains(".profile") && warning.contains("secret-looking content")
    }));
    assert_eq!(discovery["mutated"], false);
    assert_json_keys(
        &discovery,
        &["mutated", "next_actions", "services_dir", "suggestions"],
    );
    assert!(
        discovery["next_actions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "review suggestions and choose one service to add"
            ))
    );
    assert!(
        discovery["next_actions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "run lattice plan <service> before backup or restore"
            ))
    );
    assert_eq!(
        tool["next_command"],
        format!(
            "lattice service add tool --root {} --include settings.toml",
            env.home.join(".config/tool").display()
        )
    );
    assert_eq!(tool["uses_app_catalog"], false);
    assert_eq!(
        warning_only["next_command"],
        "review warnings before adding a service"
    );

    let discovery_text = run_ok(bin, &env, &["discover"]);
    assert!(discovery_text.contains("warning: excluded plain-key.toml"));
    assert!(discovery_text.contains("onlysecret root="));
    assert!(discovery_text.contains("warning: excluded config.toml"));
    assert!(discovery_text.contains("warning: excluded .profile"));
    assert!(discovery_text.contains("next command: lattice service add tool --root"));
    assert!(discovery_text.contains("next command: review warnings before adding a service"));
    assert!(discovery_text.contains("next actions:"));
    assert!(discovery_text.contains("- review suggestions and choose one service to add"));
    assert!(discovery_text.contains("- run lattice plan <service> before backup or restore"));
    assert!(!discovery_text.contains("fake_discovery_token"));
    assert!(!discovery_text.contains("fake_warning_only_token"));
    assert!(!env.config.join("lattice/services/tool.toml").exists());
}
