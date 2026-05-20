use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{CreateDirRule, PermissionRule};
use crate::manifest::{Manifest, ManifestEntry, read_manifest, write_manifest};
use crate::scanner::scan_service;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    pub copied: Vec<PathBuf>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    pub restored: Vec<PathBuf>,
}

pub fn backup_service(
    root: &Path,
    repo: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<BackupReport> {
    let files = scan_service(root, include, exclude)?;
    let mut entries = Vec::with_capacity(files.len());

    fs::create_dir_all(repo).with_context(|| format!("failed to create {}", repo.display()))?;

    for relative in &files {
        let source = root.join(relative);
        let destination = repo.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        entries.push(ManifestEntry {
            path: relative.clone(),
            mode: read_mode(&source)?,
        });
    }

    let manifest_path = repo.join(".lattice").join("manifest.toml");
    write_manifest(
        &manifest_path,
        &Manifest {
            version: 1,
            entries,
        },
    )?;

    Ok(BackupReport {
        copied: files,
        manifest_path,
    })
}

pub fn restore_service(repo: &Path, root: &Path) -> Result<RestoreReport> {
    let manifest_path = repo.join(".lattice").join("manifest.toml");
    let manifest = read_manifest(&manifest_path)?;
    let mut restored = Vec::with_capacity(manifest.entries.len());

    for entry in manifest.entries {
        let source = repo.join(&entry.path);
        let destination = root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to restore {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        apply_mode(&destination, &entry.mode)?;
        restored.push(entry.path);
    }

    restored.sort();
    Ok(RestoreReport { restored })
}

pub fn create_restore_dirs(root: &Path, dirs: &[CreateDirRule]) -> Result<Vec<PathBuf>> {
    let mut created = Vec::with_capacity(dirs.len());
    for dir in dirs {
        let path = root.join(&dir.path);
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        apply_mode(&path, &dir.mode)?;
        created.push(PathBuf::from(&dir.path));
    }
    Ok(created)
}

pub fn apply_permission_rules(root: &Path, rules: &[PermissionRule]) -> Result<Vec<PathBuf>> {
    let mut applied = Vec::new();
    for rule in rules {
        let path = root.join(&rule.path);
        if path.exists() {
            apply_mode(&path, &rule.mode)?;
            applied.push(PathBuf::from(&rule.path));
        }
    }
    Ok(applied)
}

#[cfg(unix)]
fn read_mode(path: &Path) -> Result<String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::metadata(path)
        .with_context(|| format!("failed to stat {}", path.display()))?
        .permissions()
        .mode()
        & 0o777;
    Ok(format!("{mode:04o}"))
}

#[cfg(not(unix))]
fn read_mode(path: &Path) -> Result<String> {
    fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    Ok("0644".to_string())
}

#[cfg(unix)]
fn apply_mode(path: &Path, mode: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let parsed = u32::from_str_radix(mode, 8).with_context(|| format!("invalid mode {mode}"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(parsed))
        .with_context(|| format!("failed to chmod {}", path.display()))
}

#[cfg(not(unix))]
fn apply_mode(_path: &Path, _mode: &str) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::Path;

    use tempfile::tempdir;

    use super::{backup_service, restore_service};

    #[test]
    fn backs_up_files_manifest_and_restores_modes() {
        let source = tempdir().expect("source tempdir");
        let repo = tempdir().expect("repo tempdir");
        let restore = tempdir().expect("restore tempdir");

        write_file(source.path(), "config.toml", "model = \"gpt-5.5\"\n", 0o600);
        write_file(source.path(), "bin/mcp-rbw", "#!/usr/bin/env bash\n", 0o700);
        write_file(source.path(), "auth.json", "{}\n", 0o600);
        symlink(
            source.path().join("config.toml"),
            source.path().join("bin/config-link"),
        )
        .expect("create symlink");

        let include = vec!["config.toml".to_string(), "bin/**".to_string()];
        let exclude = vec!["auth.json".to_string()];

        let backup = backup_service(source.path(), repo.path(), &include, &exclude)
            .expect("backup should work");

        assert_eq!(
            backup
                .copied
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["bin/mcp-rbw", "config.toml"]
        );
        assert!(repo.path().join("config.toml").exists());
        assert!(repo.path().join("bin/mcp-rbw").exists());
        assert!(!repo.path().join("auth.json").exists());
        assert!(!repo.path().join("bin/config-link").exists());
        assert!(repo.path().join(".lattice/manifest.toml").exists());

        let restore_report =
            restore_service(repo.path(), restore.path()).expect("restore should work");

        assert_eq!(
            restore_report
                .restored
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["bin/mcp-rbw", "config.toml"]
        );
        assert_eq!(
            fs::read_to_string(restore.path().join("config.toml")).expect("restored config"),
            "model = \"gpt-5.5\"\n"
        );
        assert_eq!(mode(restore.path().join("config.toml").as_path()), 0o600);
        assert_eq!(mode(restore.path().join("bin/mcp-rbw").as_path()), 0o700);
    }

    fn write_file(root: &Path, relative: &str, body: &str, mode: u32) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(&path, body).expect("write file");
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set permissions");
    }

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).expect("metadata").permissions().mode() & 0o777
    }
}
