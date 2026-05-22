use std::fmt::Write as _;
use std::fs;
use std::io::IsTerminal;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use inquire::Select;
use lattice_core::app_catalog::{app_names, find_app};
use lattice_core::config::{
    ConditionsConfig, GlobalConfig, HooksConfig, PermissionRule, RestoreConfig, SecretRef,
    ServiceConfig,
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
use lattice_core::secrets::find_secret_like_patterns;
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Parser)]
#[command(name = "lattice")]
#[command(about = "A small dotfiles and configuration manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Create Lattice config and storage directories")]
    Init {
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Print config paths and optional tool availability")]
    Doctor,
    #[command(about = "Validate global and service configuration")]
    Validate,
    #[command(about = "Manage service configuration entries")]
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    #[command(about = "Add or remove include globs for a service")]
    Include {
        #[command(subcommand)]
        command: PatternCommands,
    },
    #[command(about = "Add or remove exclude globs for a service")]
    Exclude {
        #[command(subcommand)]
        command: PatternCommands,
    },
    #[command(about = "Manage restore permission rules")]
    Permission {
        #[command(subcommand)]
        command: PermissionCommands,
    },
    #[command(about = "Manage app catalog shortcuts")]
    App {
        #[command(subcommand)]
        command: AppCommands,
    },
    #[command(about = "Check new-machine readiness without mutating state")]
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommands,
    },
    #[command(about = "Run git operations for a service repo")]
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
    #[command(about = "Manage secret references for a service")]
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },
    #[command(about = "Add tracked paths to a service")]
    Track {
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    #[command(about = "Copy existing local paths into a service repo")]
    Adopt {
        #[arg(long)]
        allow_secret_looking_files: bool,
        #[arg(long)]
        allow_metadata_loss: bool,
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    #[command(about = "Compare local files with the service repo")]
    Diff {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Open the interactive text UI")]
    Tui {
        #[arg(long)]
        dry_run: bool,
    },
    #[command(about = "Summarize backup and restore risk before changing files")]
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Show backup and restore status for a service")]
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Copy selected local files into a service repo")]
    Backup {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        allow_secret_looking_files: bool,
        #[arg(long)]
        allow_metadata_loss: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Restore selected files from a service repo")]
    Restore {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    #[command(about = "Inspect and prune restore safety snapshots")]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommands,
    },
    #[command(about = "Restore files from a safety snapshot")]
    Undo {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        snapshot: String,
        service: Option<String>,
    },
    #[command(about = "Suggest local service candidates without mutating state")]
    Discover {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum SnapshotCommands {
    #[command(about = "List recorded restore safety snapshots")]
    List {
        #[arg(long)]
        json: bool,
    },
    #[command(about = "Show files captured in one safety snapshot")]
    Show {
        #[arg(long)]
        json: bool,
        snapshot: String,
        service: Option<String>,
    },
    #[command(about = "Delete old safety snapshots after keeping recent entries")]
    Prune {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long, default_value_t = 20)]
        keep: usize,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    #[command(about = "List configured services")]
    List,
    #[command(about = "Print one service config file")]
    Show { service: String },
    #[command(about = "Create or overwrite a service config file")]
    Add {
        service: String,
        #[arg(long)]
        root: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        include: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        #[arg(long)]
        template: bool,
        #[arg(long)]
        symlink: bool,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Remove a service config file")]
    Remove {
        #[arg(long)]
        yes: bool,
        service: String,
    },
}

#[derive(Debug, Subcommand)]
enum PatternCommands {
    #[command(about = "Add glob patterns to the service")]
    Add {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
    #[command(about = "Remove glob patterns from the service")]
    Remove {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum PermissionCommands {
    #[command(about = "Set a restore mode for one path")]
    Set {
        service: String,
        path: String,
        mode: String,
    },
    #[command(about = "Remove a restore mode for one path")]
    Remove { service: String, path: String },
}

#[derive(Debug, Subcommand)]
enum AppCommands {
    #[command(about = "List built-in app catalog entries")]
    List,
    #[command(about = "Show the suggested config for an app")]
    Show { app: String },
    #[command(about = "Create a service config from an app catalog entry")]
    Add {
        app: String,
        #[arg(long)]
        root: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        template: bool,
        #[arg(long)]
        symlink: bool,
        #[arg(long)]
        os: Option<String>,
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum BootstrapCommands {
    #[command(about = "Report readiness and recommended next actions")]
    Check {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    #[command(about = "Show git status for a service repo")]
    Status { service: String },
    #[command(about = "Pull changes into a service repo")]
    Pull { service: String },
    #[command(about = "Commit service repo changes")]
    Commit {
        service: String,
        #[arg(short, long)]
        message: String,
    },
    #[command(about = "Push service repo changes")]
    Push { service: String },
}

#[derive(Debug, Subcommand)]
enum SecretCommands {
    #[command(about = "List secret references for a service")]
    List { service: String },
    #[command(about = "Add a secret reference to a service")]
    Add {
        service: String,
        name: String,
        #[arg(long)]
        backend: String,
        #[arg(long)]
        item: String,
        #[arg(long)]
        field: Option<String>,
        #[arg(long)]
        env: Option<String>,
        #[arg(long)]
        folder: Option<String>,
    },
    #[command(about = "Remove a secret reference from a service")]
    Remove { service: String, name: String },
    #[command(about = "Check whether configured secret backends are available")]
    Check { service: String },
}

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
        Commands::Permission { command } => update_permissions(&paths, command),
        Commands::App { command } => app_command(&paths, command),
        Commands::Bootstrap { command } => bootstrap_command(&paths, command),
        Commands::Repo { command } => repo_command(&paths, command),
        Commands::Secret { command } => secret_command(&paths, command),
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
        Commands::Tui { dry_run } => tui(&paths, dry_run),
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
        Commands::Snapshot { command } => snapshot_command(&paths, command),
        Commands::Undo {
            dry_run,
            json,
            yes,
            snapshot,
            service,
        } => undo_snapshot(&paths, &snapshot, service.as_deref(), dry_run, json, yes),
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

fn bootstrap_check(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let config_exists = paths.config_file.exists();
    let services_dir_exists = paths.services_dir.is_dir();
    let git_available = which::which("git").is_ok();
    let services = if services_dir_exists {
        load_services(paths)?
    } else {
        Vec::new()
    };
    let mut service_reports = Vec::new();
    let mut ready_count = 0usize;
    let mut any_service_issues = false;
    let mut any_service_warnings = false;
    let mut next_actions = Vec::<String>::new();

    if !config_exists {
        next_actions.push("run lattice init".to_string());
    }
    if !services_dir_exists {
        next_actions.push("create services directory".to_string());
    }
    if !git_available {
        next_actions.push("install git".to_string());
    }

    for service in services {
        let root = expand_path(&service.root)?;
        let repo = resolve_repo_path(paths, &service)?;
        let manifest = repo.join(".lattice").join("manifest.toml");
        let active = service_is_active(&service);
        let root_exists = root.exists();
        let repo_exists = repo.exists();
        let git_repo = repo.join(".git").exists();
        let remote = git_remote_status(&repo);
        let dirty = git_repo && git_dirty(&repo);
        let mut issues = Vec::<String>::new();
        let mut warnings = Vec::<String>::new();
        if !active {
            issues.push("inactive".to_string());
        }
        if !root_exists {
            issues.push("missing_root".to_string());
        }
        if !repo_exists {
            issues.push("missing_repo".to_string());
        }
        if repo_exists && !git_repo {
            warnings.push("repo_not_git".to_string());
        }
        if git_repo && remote == "missing" {
            warnings.push("missing_remote".to_string());
        }
        if dirty {
            warnings.push("dirty_repo".to_string());
        }
        if !manifest.exists() {
            issues.push("missing_manifest".to_string());
        }
        let ready = active && root_exists && manifest.exists() && issues.is_empty();
        if ready {
            ready_count += 1;
        }
        if !issues.is_empty() {
            any_service_issues = true;
        }
        if !warnings.is_empty() {
            any_service_warnings = true;
        }
        service_reports.push(serde_json::json!({
            "service": service.name,
            "active": active,
            "root": root.display().to_string(),
            "root_exists": root_exists,
            "repo": repo.display().to_string(),
            "repo_exists": repo_exists,
            "git_repo": git_repo,
            "remote": remote,
            "dirty": dirty,
            "manifest": if manifest.exists() { "present" } else { "missing" },
            "issues": issues,
            "warnings": warnings,
            "ready": ready
        }));
    }

    if any_service_issues {
        next_actions.push("create or restore missing service roots".to_string());
        next_actions.push("pull or initialize disconnected repos".to_string());
        next_actions.push("review lattice plan <service> before restore".to_string());
    }
    if any_service_warnings {
        next_actions.push("review repo warnings before sharing across machines".to_string());
    }
    next_actions.sort();
    next_actions.dedup();

    let ok = config_exists && services_dir_exists && git_available && !any_service_issues;
    if json_output {
        print_json(serde_json::json!({
            "config": paths.config_file.display().to_string(),
            "config_exists": config_exists,
            "services_dir": paths.services_dir.display().to_string(),
            "services_dir_exists": services_dir_exists,
            "diagnostics": {
                "git": if git_available { "available" } else { "missing" }
            },
            "git": if git_available { "available" } else { "missing" },
            "services": service_reports,
            "ready_services": ready_count,
            "next_actions": next_actions,
            "ok": ok
        }))?;
        return Ok(());
    }

    println!("bootstrap check");
    println!(
        "config: {} ({})",
        paths.config_file.display(),
        if config_exists { "present" } else { "missing" }
    );
    println!(
        "services: {} ({})",
        paths.services_dir.display(),
        if services_dir_exists {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "git: {}",
        if git_available {
            "available"
        } else {
            "missing"
        }
    );
    println!("ready services: {ready_count}");
    for report in service_reports {
        println!(
            "- {} active={} root={} repo={} git={} manifest={} ready={} issues={} warnings={}",
            report["service"].as_str().unwrap_or_default(),
            if report["active"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
            if report["root_exists"].as_bool().unwrap_or(false) {
                "present"
            } else {
                "missing"
            },
            if report["repo_exists"].as_bool().unwrap_or(false) {
                "present"
            } else {
                "missing"
            },
            if report["git_repo"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
            report["manifest"].as_str().unwrap_or_default(),
            if report["ready"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            },
            report["issues"].as_array().map_or(0, Vec::len),
            report["warnings"].as_array().map_or(0, Vec::len)
        );
    }
    if !next_actions.is_empty() {
        println!("next actions:");
        for action in next_actions {
            println!("- {action}");
        }
    }
    println!("ok: {}", if ok { "yes" } else { "no" });
    Ok(())
}

fn validate(paths: &LatticePaths) -> Result<()> {
    let global = load_global_config(paths)?;
    let services = load_services(paths)?;

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

fn update_permissions(paths: &LatticePaths, command: PermissionCommands) -> Result<()> {
    match command {
        PermissionCommands::Set {
            service,
            path,
            mode,
        } => {
            validate_mode(&mode)?;
            validate_relative_config_path(&path)?;
            let mut config = load_service(paths, &service)?;
            if let Some(rule) = config.permissions.iter_mut().find(|rule| rule.path == path) {
                rule.mode = mode;
            } else {
                config.permissions.push(PermissionRule { path, mode });
            }
            config
                .permissions
                .sort_by(|left, right| left.path.cmp(&right.path));
            write_service_config(paths, &config)?;
            println!("updated permission rules");
            Ok(())
        }
        PermissionCommands::Remove { service, path } => {
            let mut config = load_service(paths, &service)?;
            config.permissions.retain(|rule| rule.path != path);
            write_service_config(paths, &config)?;
            println!("removed permission rule");
            Ok(())
        }
    }
}

fn app_command(paths: &LatticePaths, command: AppCommands) -> Result<()> {
    match command {
        AppCommands::List => {
            for name in app_names() {
                println!("{name}");
            }
            Ok(())
        }
        AppCommands::Show { app } => {
            let app = find_app(&app).with_context(|| format!("unknown app {app}"))?;
            println!("app: {}", app.name);
            println!("include:");
            for pattern in app.include {
                println!("  {pattern}");
            }
            println!("exclude:");
            for pattern in app.exclude {
                println!("  {pattern}");
            }
            Ok(())
        }
        AppCommands::Add {
            app,
            root,
            repo,
            template,
            symlink,
            os,
            hostname,
            force,
        } => {
            let entry = find_app(&app).with_context(|| format!("unknown app {app}"))?;
            service_add(
                paths,
                ServiceAddInput {
                    service: entry.name.to_string(),
                    root,
                    repo,
                    include: entry.include,
                    exclude: entry.exclude,
                    template,
                    symlink,
                    os,
                    hostname,
                    force,
                },
            )
        }
    }
}

fn bootstrap_command(paths: &LatticePaths, command: BootstrapCommands) -> Result<()> {
    match command {
        BootstrapCommands::Check { json } => bootstrap_check(paths, json),
    }
}

fn repo_command(paths: &LatticePaths, command: RepoCommands) -> Result<()> {
    match command {
        RepoCommands::Status { service } => {
            run_repo_command(paths, &service, ["status", "--short", "--branch"])
        }
        RepoCommands::Pull { service } => run_repo_command(paths, &service, ["pull", "--ff-only"]),
        RepoCommands::Commit { service, message } => {
            let repo = repo_for_command(paths, &service)?;
            ensure_repo_has_no_secret_like_content(&repo)?;
            run_git_passthrough(&repo, ["add", "."])?;
            run_git_passthrough(&repo, ["commit", "-m", &message])
        }
        RepoCommands::Push { service } => {
            let repo = repo_for_command(paths, &service)?;
            ensure_repo_has_no_secret_like_content(&repo)?;
            run_git_passthrough(&repo, ["push"])
        }
    }
}

fn run_repo_command<const N: usize>(
    paths: &LatticePaths,
    service_name: &str,
    args: [&str; N],
) -> Result<()> {
    let repo = repo_for_command(paths, service_name)?;
    run_git_passthrough(&repo, args)
}

fn repo_for_command(paths: &LatticePaths, service_name: &str) -> Result<PathBuf> {
    let service = load_service(paths, service_name)?;
    let repo = resolve_repo_path(paths, &service)?;
    if !repo.exists() {
        bail!("repo does not exist: {}", repo.display());
    }
    if !repo.join(".git").exists() {
        bail!("repo is not a git repository: {}", repo.display());
    }
    Ok(repo)
}

fn run_git_passthrough<const N: usize>(repo: &Path, args: [&str; N]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .with_context(|| format!("failed to run git in {}", repo.display()))?;
    if !status.success() {
        bail!("git exited with {status}");
    }
    Ok(())
}

fn ensure_repo_has_no_secret_like_content(repo: &Path) -> Result<()> {
    let mut findings = Vec::new();
    collect_repo_secret_findings(repo, repo, &mut findings)?;
    if !findings.is_empty() {
        bail!(
            "secret-looking content found in repo: {}",
            findings.join("; ")
        );
    }
    Ok(())
}

fn collect_repo_secret_findings(root: &Path, dir: &Path, findings: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "repo contains symlink; review before commit: {}",
                path.display()
            );
        }
        if metadata.is_dir() {
            collect_repo_secret_findings(root, &path, findings)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let content = String::from_utf8_lossy(&bytes);
        let patterns = find_secret_like_patterns(&content);
        if !patterns.is_empty() {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            findings.push(format!("{} ({})", relative.display(), patterns.join(", ")));
        }
    }
    Ok(())
}

fn secret_command(paths: &LatticePaths, command: SecretCommands) -> Result<()> {
    match command {
        SecretCommands::List { service } => {
            let service = load_service(paths, &service)?;
            for secret in service.secrets {
                println!(
                    "{} backend={} item={} field={} env={} folder={}",
                    secret.name,
                    secret.backend,
                    secret.item,
                    secret.field.as_deref().unwrap_or("-"),
                    secret.env.as_deref().unwrap_or("-"),
                    secret.folder.as_deref().unwrap_or("-")
                );
            }
            Ok(())
        }
        SecretCommands::Add {
            service,
            name,
            backend,
            item,
            field,
            env,
            folder,
        } => {
            validate_secret_backend(&backend)?;
            let mut config = load_service(paths, &service)?;
            config.secrets.retain(|secret| secret.name != name);
            config.secrets.push(SecretRef {
                name,
                backend,
                item,
                field,
                env,
                folder,
            });
            config
                .secrets
                .sort_by(|left, right| left.name.cmp(&right.name));
            write_service_config(paths, &config)?;
            println!("updated secret metadata");
            Ok(())
        }
        SecretCommands::Remove { service, name } => {
            let mut config = load_service(paths, &service)?;
            config.secrets.retain(|secret| secret.name != name);
            write_service_config(paths, &config)?;
            println!("removed secret metadata");
            Ok(())
        }
        SecretCommands::Check { service } => {
            let config = load_service(paths, &service)?;
            for secret in config.secrets {
                validate_secret_backend(&secret.backend)?;
                let status = if which::which(&secret.backend).is_ok() {
                    "available"
                } else {
                    "missing"
                };
                println!(
                    "{} backend={} status={} item={} value=not-read",
                    secret.name, secret.backend, status, secret.item
                );
            }
            Ok(())
        }
    }
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

fn tui(paths: &LatticePaths, dry_run: bool) -> Result<()> {
    let actions = vec![
        "service list",
        "validate",
        "status <service>",
        "diff <service>",
        "backup --dry-run <service>",
        "plan <service>",
        "app list",
    ];
    if dry_run {
        print_tui_dashboard(paths, &actions)?;
        return Ok(());
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("interactive TUI requires a terminal; use --dry-run");
    }

    print_tui_dashboard(paths, &actions)?;
    let action = Select::new("Lattice action", actions)
        .prompt()
        .context("failed to read TUI selection")?;
    match action {
        "service list" => service_list(paths),
        "validate" => validate(paths),
        "app list" => app_command(paths, AppCommands::List),
        "status <service>" | "diff <service>" | "backup --dry-run <service>" | "plan <service>" => {
            let service = select_service_name(paths)?;
            match action {
                "status <service>" => status(paths, &service, false, PathSelection::default()),
                "diff <service>" => diff(paths, &service, false, PathSelection::default()),
                "backup --dry-run <service>" => backup(
                    paths,
                    &service,
                    BackupCommandOptions {
                        dry_run: true,
                        json_output: false,
                        yes: false,
                        allow_secret_looking_files: false,
                        allow_metadata_loss: false,
                        selection: PathSelection::default(),
                    },
                ),
                "plan <service>" => plan(paths, &service, false, PathSelection::default()),
                _ => Ok(()),
            }
        }
        _ => Ok(()),
    }
}

fn print_tui_dashboard(paths: &LatticePaths, actions: &[&str]) -> Result<()> {
    println!("lattice tui dashboard");
    println!("config: {}", paths.config_file.display());
    println!("services:");
    for service in load_services(paths)? {
        let active = if service_is_active(&service) {
            "yes"
        } else {
            "no"
        };
        let root = expand_path(&service.root);
        let repo = resolve_repo_path(paths, &service);
        let (include, exclude) = effective_patterns(&service);
        let files = match &root {
            Ok(root) => match scan_service(root, &include, &exclude) {
                Ok(files) => files.len().to_string(),
                Err(error) => format!("unavailable({error})"),
            },
            Err(error) => format!("unavailable({error})"),
        };
        println!(
            "- {} active={} files={} root={} repo={}",
            service.name,
            active,
            files,
            path_summary(&root),
            path_summary(&repo)
        );
    }
    println!("actions:");
    for action in actions {
        println!("- {action}");
    }
    Ok(())
}

fn path_summary(path: &Result<PathBuf>) -> String {
    match path {
        Ok(path) => path.display().to_string(),
        Err(error) => format!("unavailable({error})"),
    }
}

fn select_service_name(paths: &LatticePaths) -> Result<String> {
    let services = load_services(paths)?;
    let names: Vec<String> = services.into_iter().map(|service| service.name).collect();
    if names.is_empty() {
        bail!("no services configured");
    }
    Select::new("Service", names)
        .prompt()
        .context("failed to read service selection")
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

fn snapshot_command(paths: &LatticePaths, command: SnapshotCommands) -> Result<()> {
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

fn undo_snapshot(
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

    if dry_run {
        if json_output {
            print_json(serde_json::json!({
                "snapshot": record.id,
                "service": record.service,
                "dry_run": true,
                "destination": root.display().to_string(),
                "would_restore": record.entries.len(),
                "entries": record.entries
            }))?;
            return Ok(());
        }
        println!(
            "would restore {} files from snapshot {} to {}",
            record.entries.len(),
            record.id,
            root.display()
        );
        for entry in record.entries {
            println!("{entry}");
        }
        return Ok(());
    }

    let restored = restore_snapshot_entries(&record.path, &root, &record.entries)?;
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

fn discover(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
    let mut suggestions = Vec::new();
    suggestions.extend(discover_config_dirs(&home)?);
    if let Some(shell) = discover_shell(&home) {
        suggestions.push(shell);
    }
    suggestions.sort_by(|left, right| left.name.cmp(&right.name));

    if json_output {
        let values = suggestions
            .iter()
            .map(|suggestion| {
                serde_json::json!({
                    "name": suggestion.name,
                    "root": suggestion.root.display().to_string(),
                    "include": suggestion.include,
                    "exclude": suggestion.exclude,
                    "reason": suggestion.reason
                })
            })
            .collect::<Vec<_>>();
        print_json(serde_json::json!({
            "suggestions": values,
            "mutated": false,
            "services_dir": paths.services_dir.display().to_string()
        }))?;
        return Ok(());
    }

    for suggestion in suggestions {
        println!("{} root={}", suggestion.name, suggestion.root.display());
        println!("  include: {}", suggestion.include.join(", "));
        if !suggestion.exclude.is_empty() {
            println!("  exclude: {}", suggestion.exclude.join(", "));
        }
    }
    println!("mutated: no");
    Ok(())
}

#[derive(Debug, Clone)]
struct SnapshotRecord {
    id: String,
    service: String,
    path: PathBuf,
    entries: Vec<String>,
}

#[derive(Debug, Clone)]
struct DiscoverySuggestion {
    name: String,
    root: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    reason: String,
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

fn restore_snapshot_entries(
    snapshot_root: &Path,
    destination_root: &Path,
    entries: &[String],
) -> Result<usize> {
    for entry in entries {
        let relative = Path::new(entry);
        validate_relative_config_path(entry)?;
        let source = snapshot_root.join(relative);
        let destination = destination_root.join(relative);
        ensure_no_snapshot_path_symlinks(snapshot_root, relative, false)?;
        ensure_regular_snapshot_source(&source)?;
        ensure_no_snapshot_path_symlinks(destination_root, relative, true)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let source_permissions = fs::metadata(&source)
            .with_context(|| format!("failed to stat {}", source.display()))?
            .permissions();
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to restore snapshot {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        fs::set_permissions(&destination, source_permissions)
            .with_context(|| format!("failed to chmod {}", destination.display()))?;
    }
    Ok(entries.len())
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

fn snapshot_path_is_symlink(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn discover_config_dirs(home: &Path) -> Result<Vec<DiscoverySuggestion>> {
    let config = home.join(".config");
    if !config.is_dir() {
        return Ok(Vec::new());
    }
    let mut suggestions = Vec::new();
    for entry in
        fs::read_dir(&config).with_context(|| format!("failed to read {}", config.display()))?
    {
        let entry = entry?;
        let root = entry.path();
        if !snapshot_path_is_directory(&root)? {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lattice" || name.starts_with('.') {
            continue;
        }
        let (include, exclude) = conservative_patterns(&root)?;
        if include.is_empty() {
            continue;
        }
        suggestions.push(DiscoverySuggestion {
            name,
            root,
            include,
            exclude,
            reason: "local XDG config directory with non-secret-looking files".to_string(),
        });
    }
    Ok(suggestions)
}

fn discover_shell(home: &Path) -> Option<DiscoverySuggestion> {
    let mut include = [".bashrc", ".zshrc", ".profile"]
        .into_iter()
        .filter(|name| discovery_file_is_small(&home.join(name)).unwrap_or(false))
        .map(str::to_string)
        .collect::<Vec<_>>();
    include.sort();
    if include.is_empty() {
        return None;
    }
    Some(DiscoverySuggestion {
        name: "shell".to_string(),
        root: home.to_path_buf(),
        include,
        exclude: vec![
            ".cache/**".to_string(),
            ".local/share/**".to_string(),
            ".ssh/**".to_string(),
        ],
        reason: "common shell startup files".to_string(),
    })
}

fn conservative_patterns(root: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = fs::symlink_metadata(entry.path())
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let is_dir = metadata.is_dir();
        if should_exclude_discovery_name(&name, is_dir) {
            exclude.push(if is_dir { format!("{name}/**") } else { name });
            continue;
        }
        if metadata.is_file() && discovery_file_is_small(&entry.path())? {
            include.push(name);
        }
    }
    include.sort();
    include.dedup();
    exclude.sort();
    exclude.dedup();
    Ok((include, exclude))
}

fn should_exclude_discovery_name(name: &str, is_dir: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("secret")
        || lower.contains("token")
        || lower.contains("auth")
        || lower.contains("credential")
        || lower.contains("session")
        || lower.contains("cache")
        || lower.ends_with(".db")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite3")
        || (is_dir && lower == "logs")
}

fn discovery_file_is_small(path: &Path) -> Result<bool> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    Ok(!metadata.file_type().is_symlink() && metadata.is_file() && metadata.len() <= 1024 * 1024)
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
    if !matches!(backend, "rbw" | "bw") {
        bail!("secret backend must be rbw or bw");
    }

    Ok(())
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
