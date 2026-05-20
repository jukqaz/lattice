#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePreset {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

pub fn codex_preset() -> ServicePreset {
    ServicePreset {
        include: vec![
            "config.toml",
            "AGENTS.md",
            "agents/**",
            "bin/**",
            "docs/**",
            "hooks/**",
            "prompts/**",
            "rules/**",
            "skills/**",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        exclude: vec![
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
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::codex_preset;

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
        assert!(preset.exclude.contains(&"shell_snapshots/**".to_string()));
    }
}
