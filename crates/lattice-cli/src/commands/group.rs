use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lattice_core::config::{ServiceConfig, ServiceGroupConfig};
use lattice_core::ops::{PathSelection, filter_paths_by_selection, restore_plan_with_selection};
use lattice_core::paths::LatticePaths;
use lattice_core::scanner::scan_service;

use crate::{
    cli::GroupCommands,
    effective_patterns, expand_path, load_global_config, load_service, load_services,
    output::{manifest_entry_strings, path_strings, print_json},
    resolve_repo_path, selection, service_is_active, snapshot_policy,
};

pub(crate) fn run(paths: &LatticePaths, command: GroupCommands) -> Result<()> {
    match command {
        GroupCommands::List { json } => group_list(paths, json),
        GroupCommands::Show { json, group } => group_show(paths, &group, json),
        GroupCommands::Status {
            json,
            only,
            exclude,
            group,
        } => group_status(paths, &group, json, selection(only, exclude)),
        GroupCommands::Plan {
            json,
            only,
            exclude,
            group,
        } => group_plan(paths, &group, json, selection(only, exclude)),
    }
}

fn group_list(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let groups = load_groups(paths)?;
    if json_output {
        print_json(serde_json::json!({
            "groups": groups.iter().map(group_json).collect::<Vec<_>>()
        }))?;
        return Ok(());
    }

    for group in groups {
        println!("{} services={}", group.name, group.services.len());
    }
    Ok(())
}

fn group_show(paths: &LatticePaths, group_name: &str, json_output: bool) -> Result<()> {
    let group = load_group(paths, group_name)?;
    if json_output {
        print_json(group_json(&group))?;
        return Ok(());
    }

    println!("group: {}", group.name);
    if let Some(description) = &group.description {
        println!("description: {description}");
    }
    println!("services:");
    for service in &group.services {
        println!("- {service}");
    }
    Ok(())
}

fn group_status(
    paths: &LatticePaths,
    group_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let group = load_group(paths, group_name)?;
    let summaries = group
        .services
        .iter()
        .map(|service_name| service_status_summary(paths, service_name, &selection))
        .collect::<Result<Vec<_>>>()?;
    let included_files: usize = summaries
        .iter()
        .filter(|summary| summary.active)
        .map(|summary| summary.included_files.len())
        .sum();
    let active_services = summaries.iter().filter(|summary| summary.active).count();

    if json_output {
        print_json(serde_json::json!({
            "group": group.name,
            "description": group.description,
            "service_count": summaries.len(),
            "active_services": active_services,
            "included_files": included_files,
            "services": summaries.iter().map(GroupServiceStatus::json).collect::<Vec<_>>()
        }))?;
        return Ok(());
    }

    println!("group: {}", group.name);
    println!("services: {}", summaries.len());
    println!("active services: {active_services}");
    println!("included files: {included_files}");
    for summary in summaries {
        println!(
            "- {} active={} root_exists={} included_files={} manifest={}",
            summary.service,
            if summary.active { "yes" } else { "no" },
            root_exists_label(summary.root_exists),
            summary.included_files.len(),
            summary.manifest_status
        );
    }
    Ok(())
}

fn group_plan(
    paths: &LatticePaths,
    group_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let group = load_group(paths, group_name)?;
    let summaries = group
        .services
        .iter()
        .map(|service_name| service_plan_summary(paths, service_name, &selection))
        .collect::<Result<Vec<_>>>()?;
    let backup_would_copy: usize = summaries
        .iter()
        .filter(|summary| summary.active)
        .map(|summary| summary.backup_would_copy)
        .sum();
    let restore_would_restore: usize = summaries
        .iter()
        .filter(|summary| summary.active)
        .map(|summary| summary.restore_would_restore)
        .sum();
    let restore_would_create_dirs: usize = summaries
        .iter()
        .filter(|summary| summary.active)
        .map(|summary| summary.restore_would_create_dirs)
        .sum();
    let conflict_count: usize = summaries
        .iter()
        .filter(|summary| summary.active)
        .map(|summary| summary.conflicts.len())
        .sum();
    let active_services = summaries.iter().filter(|summary| summary.active).count();
    let ready = !summaries.is_empty() && summaries.iter().all(|summary| summary.ready);

    if json_output {
        print_json(serde_json::json!({
            "group": group.name,
            "description": group.description,
            "service_count": summaries.len(),
            "active_services": active_services,
            "backup_would_copy": backup_would_copy,
            "restore_would_restore": restore_would_restore,
            "restore_would_create_dirs": restore_would_create_dirs,
            "conflict_count": conflict_count,
            "conflicts": group_conflicts_json(&summaries),
            "ready": ready,
            "services": summaries.iter().map(GroupServicePlan::json).collect::<Vec<_>>()
        }))?;
        return Ok(());
    }

    println!("group plan: {}", group.name);
    println!("services: {}", summaries.len());
    println!("backup would copy: {backup_would_copy}");
    println!("restore would restore: {restore_would_restore}");
    println!("restore would create dirs: {restore_would_create_dirs}");
    println!("conflicts: {conflict_count}");
    println!("ready: {}", if ready { "yes" } else { "no" });
    for summary in summaries {
        println!(
            "- {} active={} manifest={} backup_would_copy={} restore_would_restore={} ready={}",
            summary.service,
            if summary.active { "yes" } else { "no" },
            summary.manifest_status,
            summary.backup_would_copy,
            summary.restore_would_restore,
            if summary.ready { "yes" } else { "no" }
        );
    }
    Ok(())
}

#[derive(Debug)]
struct GroupServiceStatus {
    service: String,
    root: PathBuf,
    repo: PathBuf,
    active: bool,
    root_exists: Option<bool>,
    included_files: Vec<PathBuf>,
    manifest_status: String,
}

impl GroupServiceStatus {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "service": self.service,
            "root": self.root.display().to_string(),
            "repo": self.repo.display().to_string(),
            "active": self.active,
            "root_exists": self.root_exists,
            "included_files": self.included_files.len(),
            "files": path_strings(&self.included_files),
            "manifest": self.manifest_status
        })
    }
}

#[derive(Debug)]
struct GroupServicePlan {
    service: String,
    root: PathBuf,
    repo: PathBuf,
    active: bool,
    root_exists: Option<bool>,
    manifest_status: String,
    backup_would_copy: usize,
    restore_would_restore: usize,
    restore_would_create_dirs: usize,
    conflicts: Vec<PathBuf>,
    entries: Vec<String>,
    dirs: Vec<String>,
    ready: bool,
}

impl GroupServicePlan {
    fn json(&self) -> serde_json::Value {
        let requires_force = !self.conflicts.is_empty();
        serde_json::json!({
            "service": self.service,
            "root": self.root.display().to_string(),
            "repo": self.repo.display().to_string(),
            "active": self.active,
            "root_exists": self.root_exists,
            "manifest": self.manifest_status,
            "backup_would_copy": self.backup_would_copy,
            "restore_would_restore": self.restore_would_restore,
            "restore_would_create_dirs": self.restore_would_create_dirs,
            "conflicts": path_strings(&self.conflicts),
            "entries": self.entries,
            "dirs": self.dirs,
            "safe_to_restore_without_force": !requires_force,
            "requires_force": requires_force,
            "snapshot_policy": snapshot_policy(requires_force),
            "snapshot_on_conflict": requires_force,
            "ready": self.ready
        })
    }
}

fn service_status_summary(
    paths: &LatticePaths,
    service_name: &str,
    selection: &PathSelection,
) -> Result<GroupServiceStatus> {
    let service = load_service(paths, service_name)?;
    let active = service_is_active(&service);
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;
    let root_exists = if active {
        Some(service_root_exists(&root)?)
    } else {
        None
    };
    let included_files = if root_exists == Some(true) {
        filter_paths_by_selection(scan_service(&root, &include, &exclude)?, selection)?
    } else {
        Vec::new()
    };
    let manifest = repo.join(".lattice").join("manifest.toml");
    let manifest_status = if manifest.exists() {
        "present"
    } else {
        "missing"
    }
    .to_string();
    Ok(GroupServiceStatus {
        service: service.name,
        root,
        repo,
        active,
        root_exists,
        included_files,
        manifest_status,
    })
}

fn service_plan_summary(
    paths: &LatticePaths,
    service_name: &str,
    selection: &PathSelection,
) -> Result<GroupServicePlan> {
    let service = load_service(paths, service_name)?;
    let active = service_is_active(&service);
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;
    let root_exists = if active {
        Some(service_root_exists(&root)?)
    } else {
        None
    };
    let files = if root_exists == Some(true) {
        filter_paths_by_selection(scan_service(&root, &include, &exclude)?, selection)?
    } else {
        Vec::new()
    };
    let manifest = repo.join(".lattice").join("manifest.toml");
    let manifest_exists = manifest.exists();
    let manifest_status = if manifest_exists {
        "present"
    } else {
        "missing"
    }
    .to_string();
    let (restore_would_restore, restore_would_create_dirs, conflicts, entries, dirs) =
        if active && manifest_exists {
            let plan = restore_plan_with_selection(&repo, &root, selection)?;
            (
                plan.entries.len(),
                plan.directories.len(),
                plan.conflicts,
                manifest_entry_strings(&plan.entries),
                manifest_entry_strings(&plan.directories),
            )
        } else {
            (0, 0, Vec::new(), Vec::new(), Vec::new())
        };
    let ready = active && root_exists == Some(true) && manifest_exists && conflicts.is_empty();
    Ok(GroupServicePlan {
        service: service.name,
        root,
        repo,
        active,
        root_exists,
        manifest_status,
        backup_would_copy: files.len(),
        restore_would_restore,
        restore_would_create_dirs,
        conflicts,
        entries,
        dirs,
        ready,
    })
}

fn group_json(group: &ServiceGroupConfig) -> serde_json::Value {
    serde_json::json!({
        "name": group.name,
        "description": group.description,
        "services": group.services
    })
}

fn load_groups(paths: &LatticePaths) -> Result<Vec<ServiceGroupConfig>> {
    let global = load_global_config(paths)?;
    let services = load_services(paths)?;
    validate_groups(&global.groups, &services)?;
    let mut groups = global.groups;
    groups.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(groups)
}

pub(crate) fn validate_groups(
    groups: &[ServiceGroupConfig],
    services: &[ServiceConfig],
) -> Result<()> {
    let service_names = services
        .iter()
        .map(|service| service.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut group_names = BTreeSet::new();

    for group in groups {
        if group.name.trim().is_empty() {
            bail!("group name must not be empty");
        }
        if !group_names.insert(group.name.as_str()) {
            bail!("duplicate group {}", group.name);
        }
        if group.services.is_empty() {
            bail!("group {} must include at least one service", group.name);
        }

        let mut member_names = BTreeSet::new();
        for service_name in &group.services {
            if service_name.trim().is_empty() {
                bail!("group {} includes an empty service name", group.name);
            }
            if !member_names.insert(service_name.as_str()) {
                bail!(
                    "group {} lists service {} more than once",
                    group.name,
                    service_name
                );
            }
            if !service_names.contains(service_name.as_str()) {
                bail!(
                    "group {} references unknown service {}",
                    group.name,
                    service_name
                );
            }
        }
    }

    Ok(())
}

pub(crate) fn service_root_exists(root: &Path) -> Result<bool> {
    root.try_exists()
        .with_context(|| format!("failed to inspect service root {}", root.display()))
}

fn root_exists_label(root_exists: Option<bool>) -> &'static str {
    match root_exists {
        Some(true) => "yes",
        Some(false) => "no",
        None => "skipped",
    }
}

fn group_conflicts_json(summaries: &[GroupServicePlan]) -> Vec<serde_json::Value> {
    summaries
        .iter()
        .filter(|summary| summary.active && !summary.conflicts.is_empty())
        .map(|summary| {
            serde_json::json!({
                "service": summary.service,
                "paths": path_strings(&summary.conflicts)
            })
        })
        .collect()
}

fn load_group(paths: &LatticePaths, group_name: &str) -> Result<ServiceGroupConfig> {
    load_groups(paths)?
        .into_iter()
        .find(|group| group.name == group_name)
        .with_context(|| format!("unknown group {group_name}"))
}
