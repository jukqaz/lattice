use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

pub fn scan_service(root: &Path, include: &[String], exclude: &[String]) -> Result<Vec<PathBuf>> {
    let include_set = build_globset(include).context("failed to build include globs")?;
    let exclude_set = build_globset(exclude).context("failed to build exclude globs")?;
    let mut files = Vec::new();

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk {}", root.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let relative = entry
            .path()
            .strip_prefix(root)
            .with_context(|| {
                format!(
                    "failed to make {} relative to {}",
                    entry.path().display(),
                    root.display()
                )
            })?
            .to_path_buf();

        if include_set.is_match(&relative) && !exclude_set.is_match(&relative) {
            files.push(relative);
        }
    }

    files.sort();
    Ok(files)
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).with_context(|| format!("invalid glob: {pattern}"))?);
    }
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::preset::codex_preset;

    use super::scan_service;

    #[test]
    fn scans_included_codex_files_and_skips_excluded_runtime_state() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_file(root, "config.toml", "model = \"gpt-5.5\"\n");
        write_file(root, "AGENTS.md", "# Global guidance\n");
        write_file(root, "agents/reviewer.toml", "name = \"reviewer\"\n");
        write_file(root, "bin/mcp-rbw", "#!/usr/bin/env bash\n");
        write_file(root, "skills/playwright/SKILL.md", "# Playwright\n");
        write_file(root, "auth.json", "{}\n");
        write_file(root, "sessions/current.jsonl", "{}\n");
        write_file(root, "archived_sessions/old.jsonl", "{}\n");
        write_file(root, "logs_2.sqlite", "sqlite\n");
        write_file(root, "plugins/cache/openai/plugin.json", "{}\n");
        write_file(root, "shell_snapshots/current.sh", "export SECRET=bad\n");

        let preset = codex_preset();
        let files = scan_service(root, &preset.include, &preset.exclude).expect("scan should work");
        let rels = files
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            rels,
            vec![
                "AGENTS.md",
                "agents/reviewer.toml",
                "bin/mcp-rbw",
                "config.toml",
                "skills/playwright/SKILL.md"
            ]
        );
    }

    fn write_file(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, body).expect("write file");
    }
}
