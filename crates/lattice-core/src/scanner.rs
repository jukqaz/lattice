use std::fs;
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

pub fn scan_empty_dirs(
    root: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<PathBuf>> {
    let include_set = build_globset(include).context("failed to build include globs")?;
    let exclude_set = build_globset(exclude).context("failed to build exclude globs")?;
    let mut dirs = Vec::new();

    for entry in WalkDir::new(root).follow_links(false).min_depth(1) {
        let entry = entry.with_context(|| format!("failed to walk {}", root.display()))?;
        if !entry.file_type().is_dir() {
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

        if include_set.is_match(&relative)
            && !exclude_set.is_match(&relative)
            && is_empty_dir(entry.path())?
        {
            dirs.push(relative);
        }
    }

    dirs.sort();
    Ok(dirs)
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(entries.next().is_none())
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

    use super::{scan_empty_dirs, scan_service};

    #[test]
    fn scans_included_codex_files_and_skips_excluded_runtime_state() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_file(root, "config.toml", "model = \"gpt-5.5\"\n");
        write_file(root, "AGENTS.md", "# Global guidance\n");
        write_file(root, "agents/reviewer.toml", "name = \"reviewer\"\n");
        write_file(root, "bin/mcp-rbw", "#!/usr/bin/env bash\n");
        write_file(root, "skills/playwright/SKILL.md", "# Playwright\n");
        write_file(root, "skills/.system/skill-creator/SKILL.md", "# System\n");
        fs::create_dir_all(root.join("skills/empty-skill")).expect("create empty skill");
        fs::create_dir_all(root.join("skills/full-skill")).expect("create full skill");
        write_file(root, "skills/full-skill/SKILL.md", "# Full\n");
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
                "skills/full-skill/SKILL.md",
                "skills/playwright/SKILL.md"
            ]
        );

        let dirs =
            scan_empty_dirs(root, &preset.include, &preset.exclude).expect("scan dirs should work");
        let rels = dirs
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(rels, vec!["skills/empty-skill"]);
    }

    #[test]
    #[cfg(unix)]
    fn skips_special_paths_and_symlink_only_dirs_are_not_empty() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        let temp = tempdir().expect("tempdir");
        let root = temp.path();
        write_file(root, "config.toml", "model = \"gpt-5.5\"\n");
        fs::create_dir_all(root.join("empty")).expect("create empty dir");
        fs::create_dir_all(root.join("links")).expect("create link dir");
        symlink(root.join("config.toml"), root.join("linked-config.toml"))
            .expect("create file symlink");
        symlink(root.join("missing.toml"), root.join("broken-link.toml"))
            .expect("create broken symlink");
        symlink(root.join("config.toml"), root.join("links/config.toml"))
            .expect("create nested symlink");
        let _listener = UnixListener::bind(root.join("socket")).expect("create socket");

        let include = vec!["**".to_string()];
        let exclude = Vec::new();
        let files = scan_service(root, &include, &exclude).expect("scan files should work");
        let file_rels = files
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(file_rels, vec!["config.toml"]);

        let dirs = scan_empty_dirs(root, &include, &exclude).expect("scan dirs should work");
        let dir_rels = dirs
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(dir_rels, vec!["empty"]);
    }

    fn write_file(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, body).expect("write file");
    }
}
