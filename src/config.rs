use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GlobalConfig {
    pub version: u32,
    pub profile: String,
    #[serde(default)]
    pub secrets: Option<SecretsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SecretsConfig {
    #[serde(default)]
    pub default_backend: Option<String>,
    #[serde(default)]
    pub backends: BTreeMap<String, SecretBackend>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SecretBackend {
    pub kind: String,
    #[serde(default)]
    pub bin: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ServiceConfig {
    pub name: String,
    pub root: String,
    pub repo: String,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub restore: RestoreConfig,
    #[serde(default)]
    pub permissions: Vec<PermissionRule>,
    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct RestoreConfig {
    #[serde(default)]
    pub create_dirs: Vec<CreateDirRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CreateDirRule {
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PermissionRule {
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct HooksConfig {
    #[serde(default)]
    pub before_backup: Vec<HookConfig>,
    #[serde(default)]
    pub after_backup: Vec<HookConfig>,
    #[serde(default)]
    pub before_restore: Vec<HookConfig>,
    #[serde(default)]
    pub after_restore: Vec<HookConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HookConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub timeout_sec: Option<u64>,
    #[serde(default)]
    pub confirm: bool,
}

#[cfg(test)]
mod tests {
    use super::{GlobalConfig, PermissionRule, ServiceConfig};

    #[test]
    fn parses_global_config_with_secret_backends() {
        let input = r#"
version = 1
profile = "main"

[secrets]
default_backend = "rbw"

[secrets.backends.rbw]
kind = "rbw"
bin = "rbw"

[secrets.backends.bw]
kind = "bw"
bin = "bw"
server = "https://vault.example.test"
"#;

        let config: GlobalConfig = toml::from_str(input).expect("global config should parse");

        assert_eq!(config.version, 1);
        assert_eq!(config.profile, "main");
        let secrets = config.secrets.expect("secrets section should exist");
        assert_eq!(secrets.default_backend.as_deref(), Some("rbw"));
        assert_eq!(secrets.backends["rbw"].kind, "rbw");
        assert_eq!(
            secrets.backends["bw"].server.as_deref(),
            Some("https://vault.example.test")
        );
    }

    #[test]
    fn parses_service_config_with_restore_and_permissions() {
        let input = r#"
name = "codex"
root = "~/.codex"
repo = "~/.local/share/lattice/repos/codex"
preset = "codex"
include = ["config.toml", "agents/**"]
exclude = ["auth.json", "sessions/**"]

[restore]
create_dirs = [
  { path = "shell_snapshots", mode = "0700" },
  { path = "bin", mode = "0755" },
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[permissions]]
path = "bin/mcp-rbw"
mode = "0700"
"#;

        let config: ServiceConfig = toml::from_str(input).expect("service config should parse");

        assert_eq!(config.name, "codex");
        assert_eq!(config.root, "~/.codex");
        assert_eq!(config.repo, "~/.local/share/lattice/repos/codex");
        assert_eq!(config.preset.as_deref(), Some("codex"));
        assert_eq!(config.include, vec!["config.toml", "agents/**"]);
        assert_eq!(config.exclude, vec!["auth.json", "sessions/**"]);
        assert_eq!(config.restore.create_dirs[0].path, "shell_snapshots");
        assert_eq!(config.restore.create_dirs[0].mode, "0700");
        assert_eq!(
            config.permissions[1],
            PermissionRule {
                path: "bin/mcp-rbw".to_string(),
                mode: "0700".to_string()
            }
        );
    }

    #[test]
    fn parses_service_hooks() {
        let input = r#"
name = "codex"
root = "~/.codex"
repo = "~/.local/share/lattice/repos/codex"

[[hooks.after_restore]]
name = "codex doctor"
command = "codex"
args = ["doctor", "--summary"]
timeout_sec = 60
confirm = false

[[hooks.before_backup]]
name = "confirming hook"
command = "echo"
args = ["hello"]
confirm = true
"#;

        let config: ServiceConfig = toml::from_str(input).expect("service config should parse");

        assert_eq!(config.hooks.after_restore[0].name, "codex doctor");
        assert_eq!(config.hooks.after_restore[0].command, "codex");
        assert_eq!(
            config.hooks.after_restore[0].args,
            vec!["doctor", "--summary"]
        );
        assert_eq!(config.hooks.after_restore[0].timeout_sec, Some(60));
        assert!(!config.hooks.after_restore[0].confirm);
        assert!(config.hooks.before_backup[0].confirm);
    }
}
