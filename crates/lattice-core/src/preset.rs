#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePreset {
    pub name: &'static str,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

pub fn preset_names() -> Vec<&'static str> {
    vec!["codex", "git", "mise", "ssh", "zsh"]
}

pub fn find_preset(name: &str) -> Option<ServicePreset> {
    match name {
        "codex" => Some(codex_preset()),
        "git" => Some(glob_preset("git", &[".gitconfig", ".config/git/**"], &[])),
        "mise" => Some(glob_preset(
            "mise",
            &[
                ".config/mise/config.toml",
                ".config/mise/conf.d/**",
                ".tool-versions",
            ],
            &[".local/share/mise/**", ".cache/mise/**"],
        )),
        "ssh" => Some(glob_preset(
            "ssh",
            &[".ssh/config", ".ssh/allowed_signers", ".ssh/known_hosts"],
            &[
                ".ssh/id_*",
                ".ssh/*_rsa",
                ".ssh/*_ed25519",
                ".ssh/*.pem",
                ".ssh/agent.env",
            ],
        )),
        "zsh" => Some(glob_preset(
            "zsh",
            &[".zshrc", ".zprofile", ".zshenv", ".config/zsh/**"],
            &[".zcompdump*", ".zsh_history", ".cache/**"],
        )),
        _ => None,
    }
}

pub fn codex_preset() -> ServicePreset {
    glob_preset(
        "codex",
        &[
            "config.toml",
            "AGENTS.md",
            "agents/**",
            "bin/**",
            "docs/**",
            "hooks/**",
            "prompts/**",
            "rules/**",
            "skills/**",
        ],
        &[
            "auth.json",
            "history.jsonl",
            "sessions/**",
            "archived_sessions/**",
            "log/**",
            "*.sqlite",
            "*.sqlite-shm",
            "*.sqlite-wal",
            "cache/**",
            ".tmp/**",
            "tmp/**",
            "plugins/cache/**",
            "skills/.system/**",
            "browser/**",
            "computer-use/**",
            "generated_images/**",
            "node_repl/**",
            "worktrees/**",
            "shell_snapshots/**",
            "backups/**",
            "thread_quarantine/**",
            "models_cache.json",
            "session_index.jsonl",
            ".codex-global-state.json*",
            ".DS_Store",
        ],
    )
}

fn glob_preset(name: &'static str, include: &[&str], exclude: &[&str]) -> ServicePreset {
    ServicePreset {
        name,
        include: include.iter().copied().map(String::from).collect(),
        exclude: exclude.iter().copied().map(String::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{codex_preset, find_preset, preset_names};

    #[test]
    fn codex_preset_contains_expected_config_assets_and_runtime_excludes() {
        let preset = codex_preset();

        assert!(preset.include.contains(&"config.toml".to_string()));
        assert!(preset.include.contains(&"AGENTS.md".to_string()));
        assert!(preset.include.contains(&"agents/**".to_string()));
        assert!(preset.include.contains(&"bin/**".to_string()));
        assert!(preset.include.contains(&"skills/**".to_string()));

        assert!(preset.exclude.contains(&"auth.json".to_string()));
        assert!(preset.exclude.contains(&"sessions/**".to_string()));
        assert!(preset.exclude.contains(&"archived_sessions/**".to_string()));
        assert!(preset.exclude.contains(&"*.sqlite".to_string()));
        assert!(preset.exclude.contains(&"plugins/cache/**".to_string()));
        assert!(preset.exclude.contains(&"skills/.system/**".to_string()));
        assert!(preset.exclude.contains(&"shell_snapshots/**".to_string()));
    }

    #[test]
    fn catalog_contains_expected_dotfile_presets() {
        assert_eq!(preset_names(), vec!["codex", "git", "mise", "ssh", "zsh"]);

        let ssh = find_preset("ssh").expect("ssh preset");
        assert!(ssh.include.contains(&".ssh/config".to_string()));
        assert!(ssh.exclude.contains(&".ssh/id_*".to_string()));

        let git = find_preset("git").expect("git preset");
        assert!(git.include.contains(&".gitconfig".to_string()));

        assert!(find_preset("unknown").is_none());
    }
}
