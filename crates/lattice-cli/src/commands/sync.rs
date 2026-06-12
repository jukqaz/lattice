use std::fmt::Write as _;
use std::fs;

use anyhow::Result;
use lattice_core::hooks::{HookPhase, run_hooks};
use lattice_core::ops::{
    BackupOptions, PathSelection, RestoreOptions, apply_permission_rules,
    backup_service_with_options, create_restore_dirs, filter_paths_by_selection, render_template,
    restore_plan_with_selection, restore_service_with_options,
};
use lattice_core::paths::LatticePaths;
use lattice_core::scanner::{scan_empty_dirs, scan_service};
use similar::{ChangeTag, TextDiff};

use crate::config_store::{load_service, write_service_config};
use crate::output::{
    hook_outcomes_json, manifest_entry_strings, path_strings, print_hook_outcomes, print_json,
};
use crate::runtime::expand_path;
use crate::service_state::{
    effective_patterns, ensure_service_active, normalize_values, resolve_repo_path, snapshot_policy,
};

#[derive(Debug, Clone)]
pub(crate) struct BackupCommandOptions {
    pub(crate) dry_run: bool,
    pub(crate) json_output: bool,
    pub(crate) yes: bool,
    pub(crate) allow_secret_looking_files: bool,
    pub(crate) allow_metadata_loss: bool,
    pub(crate) selection: PathSelection,
}

pub(crate) fn adopt(
    paths: &LatticePaths,
    service_name: &str,
    items: Vec<String>,
    allow_secret_looking_files: bool,
    allow_metadata_loss: bool,
) -> Result<()> {
    let mut service = load_service(paths, service_name)?;
    service.include.extend(items);
    normalize_values(&mut service.include);
    backup_service_config(
        paths,
        &service,
        BackupCommandOptions {
            dry_run: false,
            json_output: false,
            yes: false,
            allow_secret_looking_files,
            allow_metadata_loss,
            selection: PathSelection::default(),
        },
    )?;
    write_service_config(paths, &service)?;
    println!("tracked {} include patterns", service.include.len());
    Ok(())
}

pub(crate) fn diff(
    paths: &LatticePaths,
    service_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    ensure_service_active(&service)?;
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;
    let files = filter_paths_by_selection(scan_service(&root, &include, &exclude)?, &selection)?;
    let mut json_diffs = Vec::new();
    for relative in files {
        let left_path = repo.join(&relative);
        let right_path = root.join(&relative);
        if !left_path.exists() {
            if json_output {
                json_diffs.push(serde_json::json!({
                    "path": relative.display().to_string(),
                    "kind": "only_source"
                }));
            } else {
                println!("only source {}", relative.display());
            }
            continue;
        }
        let left_bytes = fs::read(&left_path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", left_path.display()))?;
        let right_bytes = fs::read(&right_path)
            .map_err(|error| anyhow::anyhow!("failed to read {}: {error}", right_path.display()))?;
        let comparable_left = if service.template {
            match String::from_utf8(left_bytes.clone()) {
                Ok(left) => render_template(&left).into_bytes(),
                Err(_) => left_bytes,
            }
        } else {
            left_bytes
        };
        if comparable_left == right_bytes {
            continue;
        }
        if !json_output {
            println!("diff {}", relative.display());
        }
        if service.template {
            if json_output {
                json_diffs.push(serde_json::json!({
                    "path": relative.display().to_string(),
                    "kind": "template"
                }));
            } else {
                println!("template-rendered content differs; line diff hidden");
            }
            continue;
        }
        match (
            String::from_utf8(comparable_left),
            String::from_utf8(right_bytes),
        ) {
            (Ok(left), Ok(right)) => {
                let diff = TextDiff::from_lines(&left, &right);
                if json_output {
                    let mut patch = String::new();
                    for change in diff.iter_all_changes() {
                        let prefix = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        write!(&mut patch, "{prefix}{change}").expect("write string diff");
                    }
                    json_diffs.push(serde_json::json!({
                        "path": relative.display().to_string(),
                        "kind": "text",
                        "patch": patch
                    }));
                } else {
                    for change in diff.iter_all_changes() {
                        let prefix = match change.tag() {
                            ChangeTag::Delete => "-",
                            ChangeTag::Insert => "+",
                            ChangeTag::Equal => " ",
                        };
                        print!("{prefix}{change}");
                    }
                }
            }
            _ => {
                if json_output {
                    json_diffs.push(serde_json::json!({
                        "path": relative.display().to_string(),
                        "kind": "binary"
                    }));
                } else {
                    println!("binary content differs; line diff hidden");
                }
            }
        }
    }
    if json_output {
        print_json(serde_json::json!({
            "service": service.name,
            "diffs": json_diffs
        }))?;
    }
    Ok(())
}

pub(crate) fn status(
    paths: &LatticePaths,
    service_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;
    let files = filter_paths_by_selection(scan_service(&root, &include, &exclude)?, &selection)?;
    let manifest = repo.join(".lattice").join("manifest.toml");
    let manifest_status = if manifest.exists() {
        "present"
    } else {
        "missing"
    };
    let active = crate::service_state::service_is_active(&service);

    if json_output {
        print_json(serde_json::json!({
            "service": service.name,
            "root": root.display().to_string(),
            "repo": repo.display().to_string(),
            "active": active,
            "included_files": files.len(),
            "files": path_strings(&files),
            "manifest": manifest_status
        }))?;
        return Ok(());
    }

    println!("service: {}", service.name);
    println!("root: {}", root.display());
    println!("repo: {}", repo.display());
    println!("active: {}", if active { "yes" } else { "no" });
    println!("included files: {}", files.len());
    println!("manifest: {manifest_status}");
    Ok(())
}

pub(crate) fn plan(
    paths: &LatticePaths,
    service_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let active = crate::service_state::service_is_active(&service);
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;
    let files = if root.exists() {
        filter_paths_by_selection(scan_service(&root, &include, &exclude)?, &selection)?
    } else {
        Vec::new()
    };
    let manifest = repo.join(".lattice").join("manifest.toml");
    let manifest_status = if manifest.exists() {
        "present"
    } else {
        "missing"
    };

    let (would_restore, would_create_dirs, conflicts, entries, dirs) = if manifest.exists() {
        let plan = restore_plan_with_selection(&repo, &root, &selection)?;
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

    let requires_force = !conflicts.is_empty();
    let ready = active && root.exists() && manifest.exists() && !requires_force;
    if json_output {
        print_json(serde_json::json!({
            "service": service.name,
            "root": root.display().to_string(),
            "repo": repo.display().to_string(),
            "active": active,
            "root_exists": root.exists(),
            "manifest": manifest_status,
            "backup_would_copy": files.len(),
            "restore_would_restore": would_restore,
            "restore_would_create_dirs": would_create_dirs,
            "conflicts": path_strings(&conflicts),
            "entries": entries,
            "dirs": dirs,
            "safe_to_restore_without_force": !requires_force,
            "requires_force": requires_force,
            "snapshot_policy": snapshot_policy(requires_force),
            "snapshot_on_conflict": requires_force,
            "ready": ready
        }))?;
        return Ok(());
    }

    println!("plan: {}", service.name);
    println!("root: {}", root.display());
    println!("repo: {}", repo.display());
    println!("active: {}", if active { "yes" } else { "no" });
    println!("root exists: {}", if root.exists() { "yes" } else { "no" });
    println!("manifest: {manifest_status}");
    println!("backup would copy: {}", files.len());
    println!("restore would restore: {would_restore}");
    println!("restore would create dirs: {would_create_dirs}");
    println!("conflicts: {}", conflicts.len());
    if !conflicts.is_empty() {
        println!("snapshot: would create before forced restore");
        for conflict in conflicts {
            println!("conflict {}", conflict.display());
        }
    }
    println!("ready: {}", if ready { "yes" } else { "no" });
    Ok(())
}

pub(crate) fn backup(
    paths: &LatticePaths,
    service_name: &str,
    options: BackupCommandOptions,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    backup_service_config(paths, &service, options)
}

pub(crate) fn backup_service_config(
    paths: &LatticePaths,
    service: &lattice_core::config::ServiceConfig,
    options: BackupCommandOptions,
) -> Result<()> {
    ensure_service_active(service)?;
    let (include, exclude) = effective_patterns(service);
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, service)?;

    if options.dry_run {
        let before_hooks = run_hooks(&service.hooks, HookPhase::BeforeBackup, true, options.yes)?;
        let files = filter_paths_by_selection(
            scan_service(&root, &include, &exclude)?,
            &options.selection,
        )?;
        let dirs = filter_paths_by_selection(
            scan_empty_dirs(&root, &include, &exclude)?,
            &options.selection,
        )?;
        let after_hooks = run_hooks(&service.hooks, HookPhase::AfterBackup, true, options.yes)?;
        if options.json_output {
            print_json(serde_json::json!({
                "service": service.name,
                "dry_run": true,
                "destination": repo.display().to_string(),
                "would_copy": files.len(),
                "would_track_dirs": dirs.len(),
                "files": path_strings(&files),
                "dirs": path_strings(&dirs),
                "hooks": hook_outcomes_json(&before_hooks, &after_hooks)
            }))?;
        } else {
            print_hook_outcomes(&before_hooks);
            println!("would copy {} files to {}", files.len(), repo.display());
            if !dirs.is_empty() {
                println!("would track {} empty dirs", dirs.len());
            }
            for file in files {
                println!("{}", file.display());
            }
            for dir in dirs {
                println!("{}/", dir.display());
            }
            print_hook_outcomes(&after_hooks);
        }
        return Ok(());
    }

    let before_hooks = run_hooks(&service.hooks, HookPhase::BeforeBackup, false, options.yes)?;
    let report = backup_service_with_options(
        &root,
        &repo,
        &include,
        &exclude,
        &BackupOptions {
            allow_secret_looking_files: options.allow_secret_looking_files,
            allow_metadata_loss: options.allow_metadata_loss,
            selection: options.selection,
        },
    )?;
    let after_hooks = run_hooks(&service.hooks, HookPhase::AfterBackup, false, options.yes)?;

    if options.json_output {
        print_json(serde_json::json!({
            "service": service.name,
            "dry_run": false,
            "destination": repo.display().to_string(),
            "copied": report.copied.len(),
            "tracked_dirs": report.created_dirs.len(),
            "files": path_strings(&report.copied),
            "dirs": path_strings(&report.created_dirs),
            "manifest": report.manifest_path.display().to_string(),
            "hooks": hook_outcomes_json(&before_hooks, &after_hooks)
        }))?;
    } else {
        print_hook_outcomes(&before_hooks);
        print_hook_outcomes(&after_hooks);
        println!("copied {} files to {}", report.copied.len(), repo.display());
        if !report.created_dirs.is_empty() {
            println!("tracked {} empty dirs", report.created_dirs.len());
        }
        println!("manifest: {}", report.manifest_path.display());
    }
    Ok(())
}

pub(crate) fn restore(
    paths: &LatticePaths,
    service_name: &str,
    dry_run: bool,
    json_output: bool,
    force: bool,
    yes: bool,
    selection: PathSelection,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    ensure_service_active(&service)?;
    let root = expand_path(&service.root)?;
    let repo = resolve_repo_path(paths, &service)?;

    if dry_run {
        let before_hooks = run_hooks(&service.hooks, HookPhase::BeforeRestore, true, yes)?;
        let plan = restore_plan_with_selection(&repo, &root, &selection)?;
        let after_hooks = run_hooks(&service.hooks, HookPhase::AfterRestore, true, yes)?;
        if json_output {
            print_json(serde_json::json!({
                "service": service.name,
                "dry_run": true,
                "destination": root.display().to_string(),
                "would_restore": plan.entries.len(),
                "would_create_dirs": plan.directories.len(),
                "entries": manifest_entry_strings(&plan.entries),
                "dirs": manifest_entry_strings(&plan.directories),
                "conflicts": path_strings(&plan.conflicts),
                "safe_to_restore_without_force": plan.conflicts.is_empty(),
                "requires_force": !plan.conflicts.is_empty(),
                "snapshot_policy": snapshot_policy(!plan.conflicts.is_empty()),
                "hooks": hook_outcomes_json(&before_hooks, &after_hooks)
            }))?;
        } else {
            print_hook_outcomes(&before_hooks);
            println!(
                "would restore {} files to {}",
                plan.entries.len(),
                root.display()
            );
            if !plan.directories.is_empty() {
                println!("would create {} empty dirs", plan.directories.len());
            }
            if !plan.conflicts.is_empty() {
                println!("conflicts: {}", plan.conflicts.len());
                for conflict in &plan.conflicts {
                    println!("conflict {}", conflict.display());
                }
            }
            for entry in plan.entries {
                println!("{}", entry.path.display());
            }
            for entry in plan.directories {
                println!("{}/", entry.path.display());
            }
            print_hook_outcomes(&after_hooks);
        }
        return Ok(());
    }

    let before_hooks = run_hooks(&service.hooks, HookPhase::BeforeRestore, false, yes)?;
    let report = restore_service_with_options(
        &repo,
        &root,
        &RestoreOptions {
            force,
            snapshot_root: Some(paths.state_dir.join("snapshots")),
            service_name: Some(service.name.clone()),
            symlink: service.restore.symlink,
            render_templates: service.template,
            selection,
        },
    )?;
    let created_dirs = create_restore_dirs(&root, &service.restore.create_dirs)?;
    let applied_permissions = apply_permission_rules(&root, &service.permissions)?;
    let after_hooks = run_hooks(&service.hooks, HookPhase::AfterRestore, false, yes)?;

    if json_output {
        print_json(serde_json::json!({
            "service": service.name,
            "dry_run": false,
            "destination": root.display().to_string(),
            "restored": report.restored.len(),
            "entries": path_strings(&report.restored),
            "created_restore_dirs": path_strings(&created_dirs),
            "created_backed_up_dirs": path_strings(&report.created_dirs),
            "applied_permissions": path_strings(&applied_permissions),
            "snapshot": report.snapshot_dir.as_ref().map(|path| path.display().to_string()),
            "hooks": hook_outcomes_json(&before_hooks, &after_hooks)
        }))?;
    } else {
        print_hook_outcomes(&before_hooks);
        print_hook_outcomes(&after_hooks);
        println!(
            "restored {} files to {}",
            report.restored.len(),
            root.display()
        );
        if !created_dirs.is_empty() {
            println!("created {} restore dirs", created_dirs.len());
        }
        if !report.created_dirs.is_empty() {
            println!("created {} backed-up empty dirs", report.created_dirs.len());
        }
        if !applied_permissions.is_empty() {
            println!("applied {} permission rules", applied_permissions.len());
        }
        if let Some(snapshot_dir) = report.snapshot_dir {
            println!("snapshot: {}", snapshot_dir.display());
        }
    }
    Ok(())
}
