use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::config::{CreateDirRule, PermissionRule};
use crate::manifest::{Manifest, ManifestEntry, read_manifest, write_manifest};
use crate::scanner::{scan_empty_dirs, scan_service};
use crate::secrets::find_secret_like_patterns;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReport {
    pub copied: Vec<PathBuf>,
    pub created_dirs: Vec<PathBuf>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackupOptions {
    pub allow_secret_looking_files: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    pub restored: Vec<PathBuf>,
    pub created_dirs: Vec<PathBuf>,
    pub conflicts: Vec<PathBuf>,
    pub snapshot_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RestoreOptions {
    pub force: bool,
    pub snapshot_root: Option<PathBuf>,
    pub service_name: Option<String>,
    pub symlink: bool,
    pub render_templates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorePlan {
    pub entries: Vec<ManifestEntry>,
    pub directories: Vec<ManifestEntry>,
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
    let dirs = scan_empty_dirs(root, include, exclude)?;
    let mut entries = Vec::with_capacity(files.len());
    let mut directories = Vec::with_capacity(dirs.len());

    if !options.allow_secret_looking_files {
        ensure_no_secret_like_files(root, &files)?;
    }

    fs::create_dir_all(repo).with_context(|| format!("failed to create {}", repo.display()))?;

    for relative in &dirs {
        let source = checked_join(root, relative)?;
        ensure_source_directory(&source)?;
        ensure_no_destination_parent_symlinks(repo, relative)?;
        let destination = checked_join(repo, relative)?;
        if destination_is_symlink(&destination)? {
            bail!("backup destination is a symlink: {}", destination.display());
        }
        fs::create_dir_all(&destination)
            .with_context(|| format!("failed to create {}", destination.display()))?;
        directories.push(ManifestEntry {
            path: relative.clone(),
            mode: read_mode(&source)?,
        });
    }

    let manifest_path = repo.join(".lattice").join("manifest.toml");
    ensure_no_destination_parent_symlinks(repo, Path::new(".lattice/manifest.toml"))?;
    if destination_is_symlink(&manifest_path)? {
        bail!(
            "backup destination is a symlink: {}",
            manifest_path.display()
        );
    }

    for relative in &files {
        let source = checked_join(root, relative)?;
        ensure_regular_source_file(&source)?;
        ensure_no_destination_parent_symlinks(repo, relative)?;
        let destination = checked_join(repo, relative)?;
        if destination_is_symlink(&destination)? {
            bail!("backup destination is a symlink: {}", destination.display());
        }
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

    write_manifest(
        &manifest_path,
        &Manifest {
            version: 1,
            directories,
            entries,
        },
    )?;

    Ok(BackupReport {
        copied: files,
        created_dirs: dirs,
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
    if manifest.version != 1 {
        bail!("unsupported manifest version: {}", manifest.version);
    }
    let mut conflicts = Vec::new();

    for entry in &manifest.directories {
        ensure_safe_relative_path(&entry.path)?;
        ensure_no_destination_parent_symlinks(root, &entry.path)?;
        let destination = checked_join(root, &entry.path)?;
        if destination_dir_conflicts(&destination)? {
            conflicts.push(entry.path.clone());
        }
    }

    for entry in &manifest.entries {
        let source = restore_source_path(repo, &entry.path)?;
        ensure_regular_source_file(&source)?;
        ensure_no_destination_parent_symlinks(root, &entry.path)?;
        let destination = checked_join(root, &entry.path)?;
        ensure_destination_file_or_absent(&destination)?;
        let destination_conflicts = destination_is_symlink(&destination)?
            || (path_exists_no_follow(&destination)? && files_differ(&source, &destination)?);
        if destination_conflicts {
            conflicts.push(entry.path.clone());
        }
    }

    conflicts.sort();
    Ok(RestorePlan {
        entries: manifest.entries,
        directories: manifest.directories,
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
    let mut created_dirs = Vec::with_capacity(plan.directories.len());
    let mut snapshot_dir = None;

    for entry in plan.directories {
        ensure_safe_relative_path(&entry.path)?;
        ensure_no_destination_parent_symlinks(root, &entry.path)?;
        let destination = checked_join(root, &entry.path)?;
        if path_exists_no_follow(&destination)? && !destination_is_directory(&destination)? {
            let snapshot = ensure_snapshot_dir(&mut snapshot_dir, options)?;
            let snapshot_path = snapshot.join(&entry.path);
            snapshot_existing_path(&destination, &snapshot_path)?;
            remove_existing_file(&destination)?;
        }
        fs::create_dir_all(&destination)
            .with_context(|| format!("failed to create {}", destination.display()))?;
        apply_mode(&destination, &entry.mode)?;
        created_dirs.push(entry.path);
    }

    for entry in plan.entries {
        let source = restore_source_path(repo, &entry.path)?;
        ensure_regular_source_file(&source)?;
        ensure_no_destination_parent_symlinks(root, &entry.path)?;
        let destination = checked_join(root, &entry.path)?;
        if path_exists_no_follow(&destination)? {
            let snapshot = ensure_snapshot_dir(&mut snapshot_dir, options)?;
            let snapshot_path = snapshot.join(&entry.path);
            snapshot_existing_path(&destination, &snapshot_path)?;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        restore_entry(&source, &destination, &entry.mode, options)?;
        restored.push(entry.path);
    }

    restored.sort();
    Ok(RestoreReport {
        restored,
        created_dirs,
        conflicts: plan.conflicts,
        snapshot_dir,
    })
}

fn restore_entry(
    source: &Path,
    destination: &Path,
    mode: &str,
    options: &RestoreOptions,
) -> Result<()> {
    if options.render_templates && restore_template(source, destination)? {
        apply_mode(destination, mode)?;
        return Ok(());
    }

    if options.symlink {
        restore_symlink(source, destination)?;
        return Ok(());
    }

    remove_existing_file(destination)?;
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to restore {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    apply_mode(destination, mode)
}

#[cfg(unix)]
fn restore_symlink(source: &Path, destination: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    remove_existing_file(destination)?;
    symlink(source, destination).with_context(|| {
        format!(
            "failed to symlink {} to {}",
            destination.display(),
            source.display()
        )
    })
}

#[cfg(not(unix))]
fn restore_symlink(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to restore {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn restore_template(source: &Path, destination: &Path) -> Result<bool> {
    let content = match fs::read_to_string(source) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", source.display()));
        }
    };
    let rendered = render_template(&content);
    if rendered == content {
        return Ok(false);
    }
    remove_existing_file(destination)?;
    fs::write(destination, rendered)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    Ok(true)
}

fn remove_existing_file(path: &Path) -> Result<()> {
    if path.exists() || fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

pub fn render_template(content: &str) -> String {
    let mut rendered = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(start) = rest.find("{{env:") {
        rendered.push_str(&rest[..start]);
        let after_start = &rest[start + "{{env:".len()..];
        if let Some(end) = after_start.find("}}") {
            let key = &after_start[..end];
            rendered.push_str(&std::env::var(key).unwrap_or_default());
            rest = &after_start[end + "}}".len()..];
        } else {
            rendered.push_str(&rest[start..]);
            return rendered;
        }
    }
    rendered.push_str(rest);
    rendered
}

pub fn create_restore_dirs(root: &Path, dirs: &[CreateDirRule]) -> Result<Vec<PathBuf>> {
    let mut created = Vec::with_capacity(dirs.len());
    for dir in dirs {
        let relative = Path::new(&dir.path);
        ensure_safe_relative_path(relative)?;
        ensure_no_destination_path_symlinks(root, relative)?;
        let path = checked_join(root, relative)?;
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
        let relative = Path::new(&rule.path);
        ensure_safe_relative_path(relative)?;
        ensure_no_destination_parent_symlinks(root, relative)?;
        let path = checked_join(root, relative)?;
        if path.exists() {
            if destination_is_symlink(&path)? {
                bail!("permission path is a symlink: {}", path.display());
            }
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

fn restore_source_path(repo: &Path, relative: &Path) -> Result<PathBuf> {
    let source = checked_join(repo, relative)?;
    ensure_no_destination_parent_symlinks(repo, relative)?;
    Ok(source)
}

fn checked_join(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure_safe_relative_path(relative)?;
    Ok(root.join(relative))
}

fn ensure_safe_relative_path(path: &Path) -> Result<()> {
    let mut has_component = false;
    for component in path.components() {
        match component {
            Component::Normal(_) => has_component = true,
            _ => bail!("unsafe relative path: {}", path.display()),
        }
    }
    if !has_component {
        bail!("unsafe relative path: {}", path.display());
    }
    Ok(())
}

fn ensure_regular_source_file(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("source path is a symlink: {}", path.display());
    }
    if !metadata.is_file() {
        bail!("source path is not a regular file: {}", path.display());
    }
    Ok(())
}

fn ensure_source_directory(path: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("source path is a symlink: {}", path.display());
    }
    if !metadata.is_dir() {
        bail!("source path is not a directory: {}", path.display());
    }
    Ok(())
}

fn ensure_no_destination_parent_symlinks(root: &Path, relative: &Path) -> Result<()> {
    let Some(parent) = relative.parent() else {
        return Ok(());
    };
    let mut current = root.to_path_buf();
    for component in parent.components() {
        current.push(component.as_os_str());
        if destination_is_symlink(&current)? {
            bail!("destination parent is a symlink: {}", current.display());
        }
    }
    Ok(())
}

fn ensure_no_destination_path_symlinks(root: &Path, relative: &Path) -> Result<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if destination_is_symlink(&current)? {
            bail!("destination parent is a symlink: {}", current.display());
        }
    }
    Ok(())
}

fn destination_is_symlink(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn path_exists_no_follow(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn ensure_destination_file_or_absent(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to stat {}", path.display()));
        }
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        return Ok(());
    }
    bail!("destination path is not a regular file: {}", path.display())
}

fn destination_dir_conflicts(path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to stat {}", path.display()));
        }
    };
    Ok(metadata.file_type().is_symlink() || !metadata.is_dir())
}

fn destination_is_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn snapshot_existing_path(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("failed to stat {}", source.display()))?;
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(source)
            .with_context(|| format!("failed to read link {}", source.display()))?;
        fs::write(destination, format!("symlink: {}\n", target.display()))
            .with_context(|| format!("failed to snapshot {}", source.display()))?;
        return Ok(());
    }
    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to snapshot {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
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
    snapshot_dir.as_deref().context("snapshot dir must exist")
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

    use crate::config::{CreateDirRule, PermissionRule};
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
        fs::create_dir_all(source.path().join("skills/empty-skill")).expect("create empty skill");
        symlink(
            source.path().join("config.toml"),
            source.path().join("bin/config-link"),
        )
        .expect("create symlink");

        let include = vec![
            "config.toml".to_string(),
            "bin/**".to_string(),
            "skills/**".to_string(),
        ];
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
        assert_eq!(
            backup
                .created_dirs
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["skills/empty-skill"]
        );
        assert!(repo.path().join("config.toml").exists());
        assert!(repo.path().join("bin/mcp-rbw").exists());
        assert!(repo.path().join("skills/empty-skill").is_dir());
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
            restore_report
                .created_dirs
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["skills/empty-skill"]
        );
        assert!(restore.path().join("skills/empty-skill").is_dir());
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
                directories: Vec::new(),
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
                directories: Vec::new(),
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
                symlink: false,
                render_templates: false,
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

    #[test]
    fn templated_restore_replaces_existing_symlink_without_rewriting_repo() {
        let repo = tempdir().expect("repo tempdir");
        let destination = tempdir().expect("destination tempdir");

        write_file(repo.path(), "config.toml", "home={{env:HOME}}\n", 0o600);
        write_manifest(
            &repo.path().join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        fs::create_dir_all(destination.path()).expect("create destination");
        symlink(
            repo.path().join("config.toml"),
            destination.path().join("config.toml"),
        )
        .expect("create existing symlink");

        restore_service_with_options(
            repo.path(),
            destination.path(),
            &RestoreOptions {
                force: true,
                snapshot_root: None,
                service_name: None,
                symlink: true,
                render_templates: true,
            },
        )
        .expect("templated symlink restore should work");

        assert!(
            !fs::symlink_metadata(destination.path().join("config.toml"))
                .expect("destination metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(destination.path().join("config.toml")).expect("destination config"),
            format!("home={}\n", std::env::var("HOME").unwrap_or_default())
        );
        assert_eq!(
            fs::read_to_string(repo.path().join("config.toml")).expect("repo config"),
            "home={{env:HOME}}\n"
        );
    }

    #[test]
    fn restore_rejects_manifest_paths_that_escape_root() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root/subroot");
        let source_outside_root = temp.path().join("escape/owned.txt");
        let destination_outside_root = temp.path().join("root/escape/owned.txt");

        fs::create_dir_all(&root).expect("create root");
        write_file(temp.path(), "escape/owned.txt", "malicious\n", 0o600);
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "../escape/owned.txt".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        assert!(source_outside_root.exists());

        let error =
            restore_service(repo.as_path(), root.as_path()).expect_err("restore should reject");

        assert!(format!("{error:#}").contains("unsafe relative path"));
        assert!(!destination_outside_root.exists());
    }

    #[test]
    fn restore_rejects_unsupported_manifest_versions() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root");

        write_file(repo.as_path(), "config.toml", "repo\n", 0o600);
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 999,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");

        let error =
            restore_service(repo.as_path(), root.as_path()).expect_err("restore should reject");

        assert!(format!("{error:#}").contains("unsupported manifest version"));
        assert!(!root.join("config.toml").exists());
    }

    #[test]
    fn restore_rejects_repo_source_symlinks() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root");
        let outside = temp.path().join("outside-secret.txt");

        write_file(temp.path(), "outside-secret.txt", "secret\n", 0o600);
        fs::create_dir_all(&repo).expect("create repo");
        symlink(&outside, repo.join("config.toml")).expect("create repo symlink");
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");

        let error =
            restore_service(repo.as_path(), root.as_path()).expect_err("restore should reject");

        assert!(format!("{error:#}").contains("source path is a symlink"));
        assert!(!root.join("config.toml").exists());
    }

    #[test]
    fn restore_rejects_destination_parent_symlink_escape() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root");
        let outside = temp.path().join("outside");

        write_file(repo.as_path(), "linked/config.toml", "safe\n", 0o600);
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "linked/config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        fs::create_dir_all(&root).expect("create root");
        fs::create_dir_all(&outside).expect("create outside");
        symlink(&outside, root.join("linked")).expect("create parent symlink");

        let error =
            restore_service(repo.as_path(), root.as_path()).expect_err("restore should reject");

        assert!(format!("{error:#}").contains("destination parent is a symlink"));
        assert!(!outside.join("config.toml").exists());
    }

    #[test]
    fn restore_does_not_snapshot_symlink_targets() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root");
        let state = temp.path().join("state");
        let outside = temp.path().join("outside-secret.txt");

        write_file(repo.as_path(), "config.toml", "repo version\n", 0o600);
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        write_file(temp.path(), "outside-secret.txt", "outside secret\n", 0o600);
        fs::create_dir_all(&root).expect("create root");
        symlink(&outside, root.join("config.toml")).expect("create destination symlink");

        let report = restore_service_with_options(
            repo.as_path(),
            root.as_path(),
            &RestoreOptions {
                force: true,
                snapshot_root: Some(state.join("snapshots")),
                service_name: Some("service".to_string()),
                symlink: false,
                render_templates: false,
            },
        )
        .expect("restore should work");

        let snapshot = report
            .snapshot_dir
            .expect("snapshot dir")
            .join("config.toml");
        assert!(
            !fs::symlink_metadata(&snapshot)
                .expect("snapshot metadata")
                .file_type()
                .is_symlink()
        );
        let snapshot_body = fs::read_to_string(&snapshot).expect("snapshot metadata content");
        assert!(snapshot_body.contains("symlink:"));
        assert!(!snapshot_body.contains("outside secret"));
        assert_eq!(
            fs::read_to_string(root.join("config.toml")).expect("restored config"),
            "repo version\n"
        );
    }

    #[test]
    fn restore_rejects_destination_directory_collision_without_deleting_it() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let root = temp.path().join("root");

        write_file(repo.as_path(), "config.toml", "repo version\n", 0o600);
        write_manifest(
            &repo.join(".lattice/manifest.toml"),
            &Manifest {
                version: 1,
                directories: Vec::new(),
                entries: vec![ManifestEntry {
                    path: "config.toml".into(),
                    mode: "0600".to_string(),
                }],
            },
        )
        .expect("write manifest");
        fs::create_dir_all(root.join("config.toml")).expect("create directory collision");

        let error = restore_service_with_options(
            repo.as_path(),
            root.as_path(),
            &RestoreOptions {
                force: true,
                snapshot_root: None,
                service_name: None,
                symlink: false,
                render_templates: false,
            },
        )
        .expect_err("restore should reject directory collision");

        assert!(format!("{error:#}").contains("destination path is not a regular file"));
        assert!(root.join("config.toml").is_dir());
    }

    #[test]
    fn restore_dir_and_permission_rules_reject_paths_that_escape_root() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("root/subroot");
        let outside_dir = temp.path().join("root/escape-dir");
        let outside_file = temp.path().join("root/escape-file");
        fs::create_dir_all(&root).expect("create root");
        write_file(temp.path(), "root/escape-file", "outside\n", 0o644);

        let dir_error = super::create_restore_dirs(
            root.as_path(),
            &[CreateDirRule {
                path: "../escape-dir".to_string(),
                mode: "0700".to_string(),
            }],
        )
        .expect_err("create dirs should reject escape");
        assert!(format!("{dir_error:#}").contains("unsafe relative path"));
        assert!(!outside_dir.exists());

        let permission_error = super::apply_permission_rules(
            root.as_path(),
            &[PermissionRule {
                path: "../escape-file".to_string(),
                mode: "0600".to_string(),
            }],
        )
        .expect_err("permission rule should reject escape");
        assert!(format!("{permission_error:#}").contains("unsafe relative path"));
        assert_eq!(mode(outside_file.as_path()), 0o644);
    }

    #[test]
    fn backup_rejects_repo_destination_symlinks() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside.txt");

        write_file(source.as_path(), "config.toml", "source\n", 0o600);
        write_file(temp.path(), "outside.txt", "outside\n", 0o600);
        fs::create_dir_all(&repo).expect("create repo");
        symlink(&outside, repo.join("config.toml")).expect("create repo destination symlink");

        let error = backup_service(
            source.as_path(),
            repo.as_path(),
            &["config.toml".to_string()],
            &[],
        )
        .expect_err("backup should reject destination symlink");

        assert!(format!("{error:#}").contains("backup destination is a symlink"));
        assert_eq!(
            fs::read_to_string(&outside).expect("outside file"),
            "outside\n"
        );
    }

    #[test]
    fn backup_rejects_repo_parent_symlink_escape() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");

        write_file(source.as_path(), "linked/config.toml", "source\n", 0o600);
        fs::create_dir_all(&repo).expect("create repo");
        fs::create_dir_all(&outside).expect("create outside");
        symlink(&outside, repo.join("linked")).expect("create repo parent symlink");

        let error = backup_service(
            source.as_path(),
            repo.as_path(),
            &["linked/config.toml".to_string()],
            &[],
        )
        .expect_err("backup should reject parent symlink");

        assert!(format!("{error:#}").contains("destination parent is a symlink"));
        assert!(!outside.join("config.toml").exists());
    }

    #[test]
    fn backup_rejects_manifest_parent_symlink_escape() {
        let temp = tempdir().expect("tempdir");
        let source = temp.path().join("source");
        let repo = temp.path().join("repo");
        let outside = temp.path().join("outside");

        write_file(source.as_path(), "config.toml", "source\n", 0o600);
        fs::create_dir_all(&repo).expect("create repo");
        fs::create_dir_all(&outside).expect("create outside");
        symlink(&outside, repo.join(".lattice")).expect("create manifest parent symlink");

        let error = backup_service(
            source.as_path(),
            repo.as_path(),
            &["config.toml".to_string()],
            &[],
        )
        .expect_err("backup should reject manifest parent symlink");

        assert!(format!("{error:#}").contains("destination parent is a symlink"));
        assert!(!repo.join("config.toml").exists());
        assert!(!outside.join("manifest.toml").exists());
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
