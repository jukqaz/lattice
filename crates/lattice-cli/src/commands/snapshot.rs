use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lattice_core::paths::LatticePaths;

use crate::cli::SnapshotCommands;
use crate::config_store::load_service;
use crate::output::print_json;
use crate::runtime::expand_path;
use crate::service_state::{ensure_service_active, validate_relative_config_path};

pub(crate) fn run(paths: &LatticePaths, command: SnapshotCommands) -> Result<()> {
    match command {
        SnapshotCommands::List { json } => snapshot_list(paths, json),
        SnapshotCommands::Show {
            json,
            snapshot,
            service,
        } => snapshot_show(paths, &snapshot, service.as_deref(), json),
        SnapshotCommands::Prune {
            dry_run,
            json,
            yes,
            keep,
        } => snapshot_prune(paths, keep, dry_run, json, yes),
    }
}

fn snapshot_list(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let snapshots = snapshot_records(paths)?;
    if json_output {
        let values = snapshots
            .iter()
            .map(|record| {
                serde_json::json!({
                    "id": record.id,
                    "service": record.service,
                    "path": record.path.display().to_string(),
                    "files": record.entries.len(),
                    "entries": record.entries
                })
            })
            .collect::<Vec<_>>();
        print_json(serde_json::json!({ "snapshots": values }))?;
        return Ok(());
    }

    for record in snapshots {
        println!(
            "{} service={} files={} path={}",
            record.id,
            record.service,
            record.entries.len(),
            record.path.display()
        );
    }
    Ok(())
}

fn snapshot_show(
    paths: &LatticePaths,
    snapshot: &str,
    service: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let record = find_snapshot(paths, snapshot, service)?;
    if json_output {
        print_json(serde_json::json!({
            "id": record.id,
            "service": record.service,
            "path": record.path.display().to_string(),
            "files": record.entries.len(),
            "entries": record.entries
        }))?;
        return Ok(());
    }

    println!("snapshot: {}", record.id);
    println!("service: {}", record.service);
    println!("path: {}", record.path.display());
    for entry in record.entries {
        println!("{entry}");
    }
    Ok(())
}

fn snapshot_prune(
    paths: &LatticePaths,
    keep: usize,
    dry_run: bool,
    json_output: bool,
    yes: bool,
) -> Result<()> {
    if !dry_run && !yes {
        bail!("snapshot prune requires --yes unless --dry-run is used");
    }
    let root = snapshot_root(paths);
    let mut ids = snapshot_ids(&root)?;
    ids.sort_by(|left, right| right.cmp(left));
    let remove = ids.into_iter().skip(keep).collect::<Vec<_>>();

    if dry_run {
        if json_output {
            print_json(serde_json::json!({
                "dry_run": true,
                "keep": keep,
                "would_remove": remove.len(),
                "remove": remove
            }))?;
            return Ok(());
        }
        println!("would remove {} snapshots", remove.len());
        for id in remove {
            println!("{id}");
        }
        return Ok(());
    }

    for id in &remove {
        let path = root.join(id);
        if snapshot_path_is_directory(&path)? {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }
    if json_output {
        print_json(serde_json::json!({
            "dry_run": false,
            "keep": keep,
            "removed": remove.len(),
            "remove": remove
        }))?;
    } else {
        println!("removed {} snapshots", remove.len());
    }
    Ok(())
}

pub(crate) fn undo(
    paths: &LatticePaths,
    snapshot: &str,
    service: Option<&str>,
    dry_run: bool,
    json_output: bool,
    yes: bool,
) -> Result<()> {
    if !dry_run && !yes {
        bail!("undo requires --yes unless --dry-run is used");
    }
    let record = find_snapshot(paths, snapshot, service)?;
    let service_config = load_service(paths, &record.service)?;
    ensure_service_active(&service_config)?;
    let root = expand_path(&service_config.root)?;
    let plan = plan_snapshot_restore_entries(&record.path, &root, &record.entries)?;

    if dry_run {
        if json_output {
            print_json(serde_json::json!({
                "snapshot": record.id,
                "service": record.service,
                "dry_run": true,
                "destination": root.display().to_string(),
                "preflight": "ok",
                "would_restore": plan.len(),
                "entries": record.entries
            }))?;
            return Ok(());
        }
        println!(
            "would restore {} files from snapshot {} to {}",
            plan.len(),
            record.id,
            root.display()
        );
        for entry in record.entries {
            println!("{entry}");
        }
        return Ok(());
    }

    let restored = restore_planned_snapshot_entries(&record.path, &root, &plan)?;
    if json_output {
        print_json(serde_json::json!({
            "snapshot": record.id,
            "service": record.service,
            "dry_run": false,
            "destination": root.display().to_string(),
            "restored": restored,
            "entries": record.entries
        }))?;
    } else {
        println!("restored {} files from snapshot {}", restored, record.id);
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct SnapshotRecord {
    id: String,
    service: String,
    path: PathBuf,
    entries: Vec<String>,
}

fn snapshot_root(paths: &LatticePaths) -> PathBuf {
    paths.state_dir.join("snapshots")
}

fn snapshot_records(paths: &LatticePaths) -> Result<Vec<SnapshotRecord>> {
    let root = snapshot_root(paths);
    let mut records = Vec::new();
    for id in snapshot_ids(&root)? {
        let id_path = root.join(&id);
        for entry in fs::read_dir(&id_path)
            .with_context(|| format!("failed to read {}", id_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !snapshot_path_is_directory(&path)? {
                continue;
            }
            let service = entry.file_name().to_string_lossy().to_string();
            let mut entries = relative_files(&path)?;
            entries.sort();
            records.push(SnapshotRecord {
                id: id.clone(),
                service,
                path,
                entries,
            });
        }
    }
    records.sort_by(|left, right| {
        right
            .id
            .cmp(&left.id)
            .then_with(|| left.service.cmp(&right.service))
    });
    Ok(records)
}

fn snapshot_ids(root: &Path) -> Result<Vec<String>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !snapshot_path_is_directory(root)? {
        bail!("snapshot root is not a directory: {}", root.display());
    }
    let mut ids = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().to_string();
        if snapshot_path_is_directory(&entry.path())? && snapshot_id_name_is_safe(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn snapshot_id_name_is_safe(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|ch| ch.is_ascii_digit())
}

fn snapshot_path_is_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn find_snapshot(
    paths: &LatticePaths,
    snapshot: &str,
    service: Option<&str>,
) -> Result<SnapshotRecord> {
    let matches = snapshot_records(paths)?
        .into_iter()
        .filter(|record| record.id == snapshot && service.is_none_or(|name| record.service == name))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [record] => Ok(record.clone()),
        [] => bail!("snapshot not found: {snapshot}"),
        _ => bail!("snapshot {snapshot} has multiple services; pass a service name"),
    }
}

fn relative_files(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    collect_relative_files(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_relative_files(root: &Path, dir: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_relative_files(root, &path, files)?;
            continue;
        }
        if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .with_context(|| format!("failed to make {} relative", path.display()))?;
            files.push(relative.to_string_lossy().to_string());
        }
    }
    Ok(())
}

fn restore_planned_snapshot_entries(
    snapshot_root: &Path,
    destination_root: &Path,
    plan: &[SnapshotRestoreEntry],
) -> Result<usize> {
    for entry in plan {
        ensure_no_snapshot_path_symlinks(snapshot_root, &entry.relative, false)?;
        ensure_regular_snapshot_source(&entry.source)?;
        create_snapshot_parent_dirs(destination_root, &entry.relative)?;
        ensure_snapshot_destination_available(destination_root, &entry.relative)?;
        let source_permissions = fs::metadata(&entry.source)
            .with_context(|| format!("failed to stat {}", entry.source.display()))?
            .permissions();
        fs::copy(&entry.source, &entry.destination).with_context(|| {
            format!(
                "failed to restore snapshot {} to {}",
                entry.source.display(),
                entry.destination.display()
            )
        })?;
        fs::set_permissions(&entry.destination, source_permissions)
            .with_context(|| format!("failed to chmod {}", entry.destination.display()))?;
    }

    Ok(plan.len())
}

struct SnapshotRestoreEntry {
    relative: PathBuf,
    source: PathBuf,
    destination: PathBuf,
}

fn plan_snapshot_restore_entries(
    snapshot_root: &Path,
    destination_root: &Path,
    entries: &[String],
) -> Result<Vec<SnapshotRestoreEntry>> {
    let mut plan = Vec::with_capacity(entries.len());

    for entry in entries {
        let relative = Path::new(entry);
        validate_relative_config_path(entry)?;
        let source = snapshot_root.join(relative);
        let destination = destination_root.join(relative);
        ensure_no_snapshot_path_symlinks(snapshot_root, relative, false)?;
        ensure_regular_snapshot_source(&source)?;
        ensure_snapshot_destination_available(destination_root, relative)?;
        plan.push(SnapshotRestoreEntry {
            relative: relative.to_path_buf(),
            source,
            destination,
        });
    }

    Ok(plan)
}

fn ensure_regular_snapshot_source(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to stat snapshot source {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("snapshot source is a symlink: {}", path.display());
    }
    if !metadata.is_file() {
        bail!("snapshot source is not a regular file: {}", path.display());
    }
    Ok(())
}

fn ensure_no_snapshot_path_symlinks(
    root: &Path,
    relative: &Path,
    include_final: bool,
) -> Result<()> {
    let mut current = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let is_final = components.peek().is_none();
        if is_final && !include_final {
            break;
        }
        current.push(component.as_os_str());
        if snapshot_path_is_symlink(&current)? {
            bail!("snapshot restore path is a symlink: {}", current.display());
        }
    }
    Ok(())
}

fn ensure_snapshot_destination_available(root: &Path, relative: &Path) -> Result<()> {
    let mut current = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let is_final = components.peek().is_none();
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    bail!("snapshot restore path is a symlink: {}", current.display());
                }
                if is_final {
                    if !metadata.is_file() {
                        bail!(
                            "snapshot restore destination is not a regular file: {}",
                            current.display()
                        );
                    }
                } else if !metadata.is_dir() {
                    bail!(
                        "snapshot restore parent is not a directory: {}",
                        current.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("failed to stat {}", current.display()));
            }
        }
    }
    Ok(())
}

fn create_snapshot_parent_dirs(root: &Path, relative: &Path) -> Result<()> {
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let mut current = root.to_path_buf();
    for component in parent.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    bail!("snapshot restore path is a symlink: {}", current.display());
                }
                if !metadata.is_dir() {
                    bail!(
                        "snapshot restore parent is not a directory: {}",
                        current.display()
                    );
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)
                    .with_context(|| format!("failed to create {}", current.display()))?;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to stat {}", current.display()));
            }
        }
    }
    Ok(())
}

fn snapshot_path_is_symlink(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}
