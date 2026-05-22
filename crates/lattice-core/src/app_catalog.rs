#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCatalogEntry {
    pub name: &'static str,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

pub fn app_names() -> Vec<&'static str> {
    vec!["codex", "git", "mise", "ssh", "zsh"]
}

pub fn find_app(name: &str) -> Option<AppCatalogEntry> {
    match name {
        "codex" => Some(codex_app()),
        "git" => Some(glob_app("git", &[".gitconfig", ".config/git/**"], &[])),
        "mise" => Some(glob_app(
            "mise",
            &[
                ".config/mise/config.toml",
                ".config/mise/conf.d/**",
                ".tool-versions",
            ],
            &[".local/share/mise/**", ".cache/mise/**"],
        )),
        "ssh" => Some(glob_app(
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
        "zsh" => Some(glob_app(
            "zsh",
            &[".zshrc", ".zprofile", ".zshenv", ".config/zsh/**"],
            &[".zcompdump*", ".zsh_history", ".cache/**"],
        )),
        _ => None,
    }
}

pub fn codex_app() -> AppCatalogEntry {
    glob_app(
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

fn glob_app(name: &'static str, include: &[&str], exclude: &[&str]) -> AppCatalogEntry {
    AppCatalogEntry {
        name,
        include: include.iter().copied().map(String::from).collect(),
        exclude: exclude.iter().copied().map(String::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{app_names, codex_app, find_app};

    #[test]
    fn codex_app_contains_expected_config_assets_and_runtime_excludes() {
        let app = codex_app();

        assert!(app.include.contains(&"config.toml".to_string()));
        assert!(app.include.contains(&"AGENTS.md".to_string()));
        assert!(app.include.contains(&"agents/**".to_string()));
        assert!(app.include.contains(&"bin/**".to_string()));
        assert!(app.include.contains(&"skills/**".to_string()));

        assert!(app.exclude.contains(&"auth.json".to_string()));
        assert!(app.exclude.contains(&"sessions/**".to_string()));
        assert!(app.exclude.contains(&"archived_sessions/**".to_string()));
        assert!(app.exclude.contains(&"*.sqlite".to_string()));
        assert!(app.exclude.contains(&"plugins/cache/**".to_string()));
        assert!(app.exclude.contains(&"skills/.system/**".to_string()));
        assert!(app.exclude.contains(&"shell_snapshots/**".to_string()));
    }

    #[test]
    fn catalog_contains_expected_dotfile_apps() {
        assert_eq!(app_names(), vec!["codex", "git", "mise", "ssh", "zsh"]);

        let ssh = find_app("ssh").expect("ssh app");
        assert!(ssh.include.contains(&".ssh/config".to_string()));
        assert!(ssh.exclude.contains(&".ssh/id_*".to_string()));

        let git = find_app("git").expect("git app");
        assert!(git.include.contains(&".gitconfig".to_string()));

        assert!(find_app("unknown").is_none());
    }
}
