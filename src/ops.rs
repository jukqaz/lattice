use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::config::{CreateDirRule, PermissionRule};
use crate::manifest::{Manifest, ManifestEntry, read_manifest, write_manifest};
use crate::scanner::scan_service;
use crate::secrets::find_secret_like_patterns;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    pub copied: Vec<PathBuf>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackupOptions {
    pub allow_secret_looking_files: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    pub restored: Vec<PathBuf>,
    pub conflicts: Vec<PathBuf>,
    pub snapshot_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreOptions {
    pub force: bool,
    pub snapshot_root: Option<PathBuf>,
    pub service_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorePlan {
    pub entries: Vec<ManifestEntry>,
    pub conflicts: Vec<PathBuf>,
}

pub fn backup_service(
    root: &Path,
    repo: &Path,
    include: &[String],
    exclude: &[String],
) -> Result<BackupReport> {
    backup_service_with_options(root, repo, include, exclude, &BackupOptions::default())
}

pub fn backup_service_with_options(
    root: &Path,
    repo: &Path,
    include: &[String],
    exclude: &[String],
    options: &BackupOptions,
) -> Result<BackupReport> {
    let files = scan_service(root, include, exclude)?;
    let mut entries = Vec::with_capacity(files.len());

    if !options.allow_secret_looking_files {
        ensure_no_secret_like_files(root, &files)?;
    }

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

fn ensure_no_secret_like_files(root: &Path, files: &[PathBuf]) -> Result<()> {
    let mut findings = Vec::new();
    for relative in files {
        let path = root.join(relative);
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let content = String::from_utf8_lossy(&bytes);
        let patterns = find_secret_like_patterns(&content);
        if !patterns.is_empty() {
            findings.push(format!("{} ({})", relative.display(), patterns.join(", ")));
        }
    }

    if !findings.is_empty() {
        bail!("secret-looking content found: {}", findings.join("; "));
    }

    Ok(())
}

pub fn restore_service(repo: &Path, root: &Path) -> Result<RestoreReport> {
    restore_service_with_options(repo, root, &RestoreOptions::default())
}

pub fn restore_plan(repo: &Path, root: &Path) -> Result<RestorePlan> {
    let manifest_path = repo.join(".lattice").join("manifest.toml");
    let manifest = read_manifest(&manifest_path)?;
    let mut conflicts = Vec::new();

    for entry in &manifest.entries {
        let source = repo.join(&entry.path);
        let destination = root.join(&entry.path);
        if destination.exists() && files_differ(&source, &destination)? {
            conflicts.push(entry.path.clone());
        }
    }

    conflicts.sort();
    Ok(RestorePlan {
        entries: manifest.entries,
        conflicts,
    })
}

pub fn restore_service_with_options(
    repo: &Path,
    root: &Path,
    options: &RestoreOptions,
) -> Result<RestoreReport> {
    let plan = restore_plan(repo, root)?;
    if !options.force && !plan.conflicts.is_empty() {
        bail!("restore conflicts: {}", join_paths(&plan.conflicts));
    }

    let mut restored = Vec::with_capacity(plan.entries.len());
    let mut snapshot_dir = None;

    for entry in plan.entries {
        let source = repo.join(&entry.path);
        let destination = root.join(&entry.path);
        if destination.exists() {
            let snapshot = ensure_snapshot_dir(&mut snapshot_dir, options)?;
            let snapshot_path = snapshot.join(&entry.path);
            if let Some(parent) = snapshot_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(&destination, &snapshot_path).with_context(|| {
                format!(
                    "failed to snapshot {} to {}",
                    destination.display(),
                    snapshot_path.display()
                )
            })?;
        }
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
    Ok(RestoreReport {
        restored,
        conflicts: plan.conflicts,
        snapshot_dir,
    })
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

fn files_differ(left: &Path, right: &Path) -> Result<bool> {
    let left = fs::read(left).with_context(|| format!("failed to read {}", left.display()))?;
    let right = fs::read(right).with_context(|| format!("failed to read {}", right.display()))?;
    Ok(left != right)
}

fn ensure_snapshot_dir<'a>(
    snapshot_dir: &'a mut Option<PathBuf>,
    options: &RestoreOptions,
) -> Result<&'a Path> {
    if snapshot_dir.is_none() {
        let root = options
            .snapshot_root
            .clone()
            .unwrap_or_else(|| PathBuf::from(".lattice-snapshots"));
        let service = options.service_name.as_deref().unwrap_or("service");
        let dir = root.join(snapshot_id()).join(service);
        fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
        *snapshot_dir = Some(dir);
    }
    Ok(snapshot_dir.as_deref().expect("snapshot dir must exist"))
}

fn snapshot_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("{millis}")
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::Path;

    use tempfile::tempdir;

    use crate::manifest::{Manifest, ManifestEntry, write_manifest};

    use super::{RestoreOptions, backup_service, restore_service, restore_service_with_options};

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

    #[test]
    fn restore_refuses_conflicting_existing_files_by_default() {
        let repo = tempdir().expect("repo tempdir");
        let destination = tempdir().expect("destination tempdir");

        write_file(repo.path(), "config.toml", "repo version\n", 0o600);
        write_manifest(
            &repo.path().join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        write_file(destination.path(), "config.toml", "local version\n", 0o600);

        let error =
            restore_service(repo.path(), destination.path()).expect_err("restore conflicts");

        assert!(format!("{error:#}").contains("restore conflicts"));
        assert_eq!(
            fs::read_to_string(destination.path().join("config.toml")).expect("destination config"),
            "local version\n"
        );
    }

    #[test]
    fn force_restore_overwrites_conflicts_and_snapshots_existing_files() {
        let repo = tempdir().expect("repo tempdir");
        let destination = tempdir().expect("destination tempdir");
        let state = tempdir().expect("state tempdir");

        write_file(repo.path(), "config.toml", "repo version\n", 0o600);
        write_manifest(
            &repo.path().join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        write_file(destination.path(), "config.toml", "local version\n", 0o600);

        let report = restore_service_with_options(
            repo.path(),
            destination.path(),
            &RestoreOptions {
                force: true,
                snapshot_root: Some(state.path().join("snapshots")),
                service_name: Some("codex".to_string()),
            },
        )
        .expect("force restore should work");

        let snapshot_dir = report.snapshot_dir.expect("snapshot dir");
        assert!(snapshot_dir.starts_with(state.path().join("snapshots")));
        assert_eq!(
            fs::read_to_string(snapshot_dir.join("config.toml")).expect("snapshot config"),
            "local version\n"
        );
        assert_eq!(
            fs::read_to_string(destination.path().join("config.toml")).expect("restored config"),
            "repo version\n"
        );
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
