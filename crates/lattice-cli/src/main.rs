use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Parser;
use lattice_core::config::{
    ConditionsConfig, GlobalConfig, HooksConfig, RestoreConfig, SecretRef, ServiceConfig,
};
use lattice_core::hooks::{HookOutcome, HookPhase, HookStatus, run_hooks};
use lattice_core::manifest::ManifestEntry;
use lattice_core::ops::{
    BackupOptions, PathSelection, RestoreOptions, apply_permission_rules,
    backup_service_with_options, create_restore_dirs, filter_paths_by_selection, render_template,
    restore_plan_with_selection, restore_service_with_options,
};
use lattice_core::paths::LatticePaths;
use lattice_core::scanner::{scan_empty_dirs, scan_service};
use similar::{ChangeTag, TextDiff};

mod cli;
mod commands;

use cli::{Cli, Commands, PatternCommands, ServiceCommands};
use commands::discover::discover;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = LatticePaths::discover()?;

    match cli.command {
        Commands::Init { force } => init(&paths, force),
        Commands::Doctor => doctor(&paths),
        Commands::Validate => validate(&paths),
        Commands::Service {
            command: ServiceCommands::List,
        } => service_list(&paths),
        Commands::Service {
            command: ServiceCommands::Show { service },
        } => service_show(&paths, &service),
        Commands::Service {
            command:
                ServiceCommands::Add {
                    service,
                    root,
                    repo,
                    include,
                    exclude,
                    template,
                    symlink,
                    os,
                    hostname,
                    force,
                },
        } => service_add(
            &paths,
            ServiceAddInput {
                service,
                root,
                repo,
                include,
                exclude,
                template,
                symlink,
                os,
                hostname,
                force,
            },
        ),
        Commands::Service {
            command: ServiceCommands::Remove { yes, service },
        } => service_remove(&paths, &service, yes),
        Commands::Include { command } => update_patterns(&paths, command, PatternTarget::Include),
        Commands::Exclude { command } => update_patterns(&paths, command, PatternTarget::Exclude),
        Commands::Permission { command } => commands::permission::run(&paths, command),
        Commands::App { command } => commands::app::run(&paths, command),
        Commands::Group { command } => commands::group::run(&paths, command),
        Commands::Bootstrap { command } => commands::bootstrap::run(&paths, command),
        Commands::Repo { command } => commands::repo::run(&paths, command),
        Commands::Secret { command } => commands::secret::run(&paths, command),
        Commands::Track {
            service,
            paths: items,
        } => track(&paths, &service, items),
        Commands::Adopt {
            allow_secret_looking_files,
            allow_metadata_loss,
            service,
            paths: items,
        } => adopt(
            &paths,
            &service,
            items,
            allow_secret_looking_files,
            allow_metadata_loss,
        ),
        Commands::Diff {
            json,
            only,
            exclude,
            service,
        } => diff(&paths, &service, json, selection(only, exclude)),
        Commands::Tui { dry_run } => commands::tui::run(&paths, dry_run),
        Commands::Plan {
            json,
            only,
            exclude,
            service,
        } => plan(&paths, &service, json, selection(only, exclude)),
        Commands::Status {
            json,
            only,
            exclude,
            service,
        } => status(&paths, &service, json, selection(only, exclude)),
        Commands::Backup {
            dry_run,
            json,
            yes,
            allow_secret_looking_files,
            allow_metadata_loss,
            only,
            exclude,
            service,
        } => backup(
            &paths,
            &service,
            BackupCommandOptions {
                dry_run,
                json_output: json,
                yes,
                allow_secret_looking_files,
                allow_metadata_loss,
                selection: selection(only, exclude),
            },
        ),
        Commands::Restore {
            dry_run,
            json,
            force,
            yes,
            only,
            exclude,
            service,
        } => restore(
            &paths,
            &service,
            dry_run,
            json,
            force,
            yes,
            selection(only, exclude),
        ),
        Commands::Snapshot { command } => commands::snapshot::run(&paths, command),
        Commands::Undo {
            dry_run,
            json,
            yes,
            snapshot,
            service,
        } => commands::snapshot::undo(&paths, &snapshot, service.as_deref(), dry_run, json, yes),
        Commands::Discover { json } => discover(&paths, json),
    }
}

#[derive(Debug, Clone)]
struct BackupCommandOptions {
    dry_run: bool,
    json_output: bool,
    yes: bool,
    allow_secret_looking_files: bool,
    allow_metadata_loss: bool,
    selection: PathSelection,
}

fn selection(only: Vec<String>, exclude: Vec<String>) -> PathSelection {
    PathSelection { only, exclude }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn present_missing(value: bool) -> &'static str {
    if value { "present" } else { "missing" }
}

fn available_missing(value: bool) -> &'static str {
    if value { "available" } else { "missing" }
}

fn init(paths: &LatticePaths, force: bool) -> Result<()> {
    let config_dir = paths
        .config_file
        .parent()
        .context("config file has no parent")?;
    fs::create_dir_all(config_dir)
        .with_context(|| format!("failed to create {}", config_dir.display()))?;
    fs::create_dir_all(&paths.services_dir)
        .with_context(|| format!("failed to create {}", paths.services_dir.display()))?;
    fs::create_dir_all(&paths.repo_cache_dir)
        .with_context(|| format!("failed to create {}", paths.repo_cache_dir.display()))?;
    fs::create_dir_all(&paths.state_dir)
        .with_context(|| format!("failed to create {}", paths.state_dir.display()))?;
    fs::create_dir_all(&paths.cache_dir)
        .with_context(|| format!("failed to create {}", paths.cache_dir.display()))?;

    write_file_if_allowed(&paths.config_file, DEFAULT_GLOBAL_CONFIG, force)?;

    println!("initialized {}", config_dir.display());
    println!("add a service with: lattice service add <name> --root <path> --include <pattern>");
    println!("next steps:");
    println!("  lattice app list");
    println!("  lattice app add <app> --root <path>");
    println!("  lattice bootstrap check");
    println!("  lattice plan <service>");
    println!("  lattice restore --dry-run <service>");
    Ok(())
}

fn doctor(paths: &LatticePaths) -> Result<()> {
    println!("config: {}", paths.config_file.display());
    println!("services: {}", paths.services_dir.display());
    println!("repos: {}", paths.repo_cache_dir.display());
    println!("state: {}", paths.state_dir.display());
    println!("cache: {}", paths.cache_dir.display());
    println!("rbw: {}", availability("rbw"));
    println!("bw: {}", availability("bw"));
    Ok(())
}

fn validate(paths: &LatticePaths) -> Result<()> {
    let global = load_global_config(paths)?;
    let services = load_services(paths)?;
    commands::group::validate_groups(&global.groups, &services)?;

    for service in &services {
        let _ = resolve_repo_path(paths, service)?;
    }

    println!("valid config");
    println!("profile: {}", global.profile);
    println!("services: {}", services.len());
    Ok(())
}

fn service_list(paths: &LatticePaths) -> Result<()> {
    for service in load_services(paths)? {
        println!("{}", service.name);
    }
    Ok(())
}

fn service_show(paths: &LatticePaths, service_name: &str) -> Result<()> {
    let path = service_file_path(paths, service_name)?;
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    print!("{body}");
    Ok(())
}

struct ServiceAddInput {
    service: String,
    root: String,
    repo: Option<String>,
    include: Vec<String>,
    exclude: Vec<String>,
    template: bool,
    symlink: bool,
    os: Option<String>,
    hostname: Option<String>,
    force: bool,
}

fn service_add(paths: &LatticePaths, input: ServiceAddInput) -> Result<()> {
    let path = service_file_path(paths, &input.service)?;
    if path.exists() && !input.force {
        bail!(
            "service {} already exists; use --force to overwrite {}",
            input.service,
            path.display()
        );
    }
    let mut include = input.include;
    let mut exclude = input.exclude;
    normalize_values(&mut include);
    normalize_values(&mut exclude);

    let service = ServiceConfig {
        name: input.service,
        root: input.root,
        repo: input.repo,
        include,
        exclude,
        template: input.template,
        conditions: ConditionsConfig {
            os: input.os,
            hostname: input.hostname,
        },
        restore: RestoreConfig {
            create_dirs: Vec::new(),
            symlink: input.symlink,
        },
        permissions: Vec::new(),
        secrets: Vec::new(),
        hooks: HooksConfig::default(),
    };
    write_service_config(paths, &service)?;
    println!("added service {}", service.name);
    Ok(())
}

fn service_remove(paths: &LatticePaths, service_name: &str, yes: bool) -> Result<()> {
    let path = service_file_path(paths, service_name)?;
    if !yes {
        bail!("removing service {service_name} requires --yes");
    }
    fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    println!("removed service {service_name}");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PatternTarget {
    Include,
    Exclude,
}

fn update_patterns(
    paths: &LatticePaths,
    command: PatternCommands,
    target: PatternTarget,
) -> Result<()> {
    let (service_name, patterns, remove) = match command {
        PatternCommands::Add { service, patterns } => (service, patterns, false),
        PatternCommands::Remove { service, patterns } => (service, patterns, true),
    };

    let mut service = load_service(paths, &service_name)?;
    let values = match target {
        PatternTarget::Include => &mut service.include,
        PatternTarget::Exclude => &mut service.exclude,
    };

    if remove {
        values.retain(|value| !patterns.contains(value));
        normalize_values(values);
        write_service_config(paths, &service)?;
        println!(
            "removed {} {} patterns",
            patterns.len(),
            pattern_target_label(target)
        );
        return Ok(());
    }

    values.extend(patterns);
    normalize_values(values);
    write_service_config(paths, &service)?;
    println!("updated {} patterns", pattern_target_label(target));
    Ok(())
}

fn track(paths: &LatticePaths, service_name: &str, items: Vec<String>) -> Result<()> {
    let mut service = load_service(paths, service_name)?;
    service.include.extend(items);
    normalize_values(&mut service.include);
    write_service_config(paths, &service)?;
    println!("tracked {} include patterns", service.include.len());
    Ok(())
}

fn adopt(
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

fn diff(
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
            .with_context(|| format!("failed to read {}", left_path.display()))?;
        let right_bytes = fs::read(&right_path)
            .with_context(|| format!("failed to read {}", right_path.display()))?;
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

fn status(
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
    let active = service_is_active(&service);

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

fn plan(
    paths: &LatticePaths,
    service_name: &str,
    json_output: bool,
    selection: PathSelection,
) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let active = service_is_active(&service);
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

fn backup(paths: &LatticePaths, service_name: &str, options: BackupCommandOptions) -> Result<()> {
    let service = load_service(paths, service_name)?;
    backup_service_config(paths, &service, options)
}

fn backup_service_config(
    paths: &LatticePaths,
    service: &ServiceConfig,
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

fn restore(
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

fn git_remote_status(repo: &Path) -> String {
    if !repo.join(".git").exists() {
        return "missing".to_string();
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", "origin"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "missing".to_string(),
    }
}

fn git_dirty(repo: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["status", "--porcelain"])
        .output();
    output
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

fn snapshot_policy(requires_force: bool) -> &'static str {
    if requires_force {
        "forced restore snapshots conflicts before overwrite"
    } else {
        "no snapshot needed for non-conflicting restore"
    }
}

fn path_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

fn manifest_entry_strings(entries: &[ManifestEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry.path.display().to_string())
        .collect()
}

fn hook_outcomes_json(before: &[HookOutcome], after: &[HookOutcome]) -> Vec<serde_json::Value> {
    before
        .iter()
        .chain(after.iter())
        .map(|outcome| {
            let status = match outcome.status {
                HookStatus::WouldRun => "would_run",
                HookStatus::Ran => "ran",
                HookStatus::SkippedConfirm => "skipped_confirm",
            };
            serde_json::json!({
                "phase": outcome.phase.label(),
                "name": outcome.name,
                "status": status
            })
        })
        .collect()
}

fn print_json(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn print_hook_outcomes(outcomes: &[HookOutcome]) {
    for outcome in outcomes {
        match outcome.status {
            HookStatus::WouldRun => {
                println!("would run hook {}: {}", outcome.phase.label(), outcome.name);
            }
            HookStatus::Ran => {
                println!("ran hook {}: {}", outcome.phase.label(), outcome.name);
            }
            HookStatus::SkippedConfirm => {
                println!(
                    "skipped hook {}: {} (requires --yes)",
                    outcome.phase.label(),
                    outcome.name
                );
            }
        }
    }
}

fn write_file_if_allowed(path: &Path, body: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    write_private_file(path, body)
}

fn write_private_file(path: &Path, body: &str) -> Result<()> {
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))?;
    set_private_file_mode(path)
}

#[cfg(unix)]
fn set_private_file_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("failed to chmod {}", path.display()))
}

#[cfg(not(unix))]
fn set_private_file_mode(_path: &Path) -> Result<()> {
    Ok(())
}

fn load_service(paths: &LatticePaths, service_name: &str) -> Result<ServiceConfig> {
    let path = service_file_path(paths, service_name)?;
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let service: ServiceConfig =
        toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))?;
    ensure_service_config_name_matches(&path, service_name, &service)?;
    Ok(service)
}

fn write_service_config(paths: &LatticePaths, service: &ServiceConfig) -> Result<()> {
    let path = service_file_path(paths, &service.name)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut body = toml::to_string_pretty(service).context("failed to serialize service config")?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    write_private_file(&path, &body)
}

fn service_file_path(paths: &LatticePaths, service_name: &str) -> Result<PathBuf> {
    Ok(paths
        .services_dir
        .join(format!("{}.toml", default_repo_dir_name(service_name)?)))
}

fn load_global_config(paths: &LatticePaths) -> Result<GlobalConfig> {
    let body = fs::read_to_string(&paths.config_file)
        .with_context(|| format!("failed to read {}", paths.config_file.display()))?;
    toml::from_str(&body)
        .with_context(|| format!("failed to parse {}", paths.config_file.display()))
}

fn load_services(paths: &LatticePaths) -> Result<Vec<ServiceConfig>> {
    if !paths.services_dir.exists() {
        return Ok(Vec::new());
    }

    let mut services = Vec::new();
    for entry in fs::read_dir(&paths.services_dir)
        .with_context(|| format!("failed to read {}", paths.services_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let service: ServiceConfig =
            toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))?;
        let expected = path
            .file_stem()
            .and_then(|value| value.to_str())
            .with_context(|| format!("service file has invalid name: {}", path.display()))?;
        ensure_service_config_name_matches(&path, expected, &service)?;
        services.push(service);
    }

    services.sort_by(|left: &ServiceConfig, right| left.name.cmp(&right.name));
    Ok(services)
}

fn effective_patterns(service: &ServiceConfig) -> (Vec<String>, Vec<String>) {
    let mut include = Vec::new();
    let mut exclude = Vec::new();

    include.extend(service.include.clone());
    exclude.extend(service.exclude.clone());
    include.sort();
    include.dedup();
    exclude.sort();
    exclude.dedup();
    (include, exclude)
}

fn resolve_repo_path(paths: &LatticePaths, service: &ServiceConfig) -> Result<PathBuf> {
    match service.repo.as_deref() {
        Some(repo) => expand_path(repo),
        None => Ok(paths
            .repo_cache_dir
            .join(default_repo_dir_name(&service.name)?)),
    }
}

fn default_repo_dir_name(service_name: &str) -> Result<&str> {
    if service_name.is_empty()
        || service_name == "."
        || service_name == ".."
        || service_name.contains('/')
        || service_name.contains('\\')
    {
        bail!("service name {service_name:?} cannot be used as a default repo directory");
    }

    Ok(service_name)
}

fn validate_secret_backend(backend: &str) -> Result<()> {
    if !matches!(backend, "rbw" | "bw" | "env") {
        bail!("secret backend must be rbw, bw, or env");
    }

    Ok(())
}

fn secret_item_for_backend(
    backend: &str,
    item: Option<String>,
    env: Option<&str>,
) -> Result<String> {
    match backend {
        "env" => Ok(env
            .map(str::to_string)
            .or(item)
            .context("env secret references require --env or --item")?),
        "rbw" | "bw" => Ok(item.context("rbw and bw secret references require --item")?),
        _ => bail!("secret backend must be rbw, bw, or env"),
    }
}

fn secret_backend_status(secret: &SecretRef) -> &'static str {
    if secret.backend == "env" {
        return match secret.env.as_deref() {
            Some(env) if std::env::var_os(env).is_some() => "set",
            Some(_) => "unset",
            None => "missing-env-reference",
        };
    }

    if which::which(&secret.backend).is_ok() {
        "available"
    } else {
        "missing"
    }
}

fn ensure_service_active(service: &ServiceConfig) -> Result<()> {
    if !service_is_active(service) {
        bail!("service {} is inactive on this host", service.name);
    }

    Ok(())
}

fn service_is_active(service: &ServiceConfig) -> bool {
    if service
        .conditions
        .os
        .as_deref()
        .is_some_and(|os| os != std::env::consts::OS)
    {
        return false;
    }
    if service
        .conditions
        .hostname
        .as_deref()
        .is_some_and(|hostname| hostname != current_hostname().as_deref().unwrap_or_default())
    {
        return false;
    }
    true
}

fn current_hostname() -> Option<String> {
    std::env::var("HOSTNAME").ok().or_else(|| {
        Command::new("hostname").output().ok().and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
    })
}

fn validate_mode(mode: &str) -> Result<()> {
    let parsed = u32::from_str_radix(mode, 8).with_context(|| format!("invalid mode {mode}"))?;
    if mode.len() != 4 || parsed > 0o777 {
        bail!("mode must be a four-digit file permission such as 0600 or 0755");
    }

    Ok(())
}

fn validate_relative_config_path(path: &str) -> Result<()> {
    let path = Path::new(path);
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

fn ensure_service_config_name_matches(
    path: &Path,
    expected_name: &str,
    service: &ServiceConfig,
) -> Result<()> {
    if service.name != expected_name {
        bail!(
            "service config name mismatch in {}: file expects {}, config has {}",
            path.display(),
            expected_name,
            service.name
        );
    }
    Ok(())
}

fn normalize_values(values: &mut Vec<String>) {
    values.retain(|value| !value.is_empty());
    values.sort();
    values.dedup();
}

fn pattern_target_label(target: PatternTarget) -> &'static str {
    match target {
        PatternTarget::Include => "include",
        PatternTarget::Exclude => "exclude",
    }
}

fn expand_path(path: &str) -> Result<PathBuf> {
    if path == "~" {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        return Ok(PathBuf::from(home));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        return Ok(PathBuf::from(home).join(rest));
    }

    Ok(PathBuf::from(path))
}

fn availability(bin: &str) -> &'static str {
    let available = which::which(bin).is_ok()
        && Command::new(bin)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

    if available { "available" } else { "missing" }
}

const DEFAULT_GLOBAL_CONFIG: &str = r#"version = 1
profile = "main"

[secrets]
default_backend = "rbw"

[secrets.backends.rbw]
kind = "rbw"
bin = "rbw"

[secrets.backends.bw]
kind = "bw"
bin = "bw"
"#;
