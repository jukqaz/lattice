use std::fmt::Write as _;
use std::fs;
use std::io::IsTerminal;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use inquire::Select;
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
use lattice_core::preset::{find_preset, preset_names};
use lattice_core::scanner::{scan_empty_dirs, scan_service};
use lattice_core::secrets::find_secret_like_patterns;
use similar::{ChangeTag, TextDiff};

#[derive(Debug, Parser)]
#[command(name = "lattice")]
#[command(about = "A small dotfiles backup and restore manager")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        #[arg(long)]
        force: bool,
    },
    Doctor,
    Validate,
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    Include {
        #[command(subcommand)]
        command: PatternCommands,
    },
    Exclude {
        #[command(subcommand)]
        command: PatternCommands,
    },
    Permission {
        #[command(subcommand)]
        command: PermissionCommands,
    },
    Preset {
        #[command(subcommand)]
        command: PresetCommands,
    },
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },
    Track {
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    Adopt {
        #[arg(long)]
        allow_secret_looking_files: bool,
        #[arg(long)]
        allow_metadata_loss: bool,
        service: String,
        #[arg(required = true)]
        paths: Vec<String>,
    },
    Diff {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
    Tui {
        #[arg(long)]
        dry_run: bool,
    },
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long, action = clap::ArgAction::Append)]
        only: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        exclude: Vec<String>,
        service: String,
    },
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
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    List,
    Show {
        service: String,
    },
    Add {
        service: String,
        #[arg(long)]
        root: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        preset: Option<String>,
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
    Remove {
        #[arg(long)]
        yes: bool,
        service: String,
    },
}

#[derive(Debug, Subcommand)]
enum PatternCommands {
    Add {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
    Remove {
        service: String,
        #[arg(required = true)]
        patterns: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum PermissionCommands {
    Set {
        service: String,
        path: String,
        mode: String,
    },
    Remove {
        service: String,
        path: String,
    },
}

#[derive(Debug, Subcommand)]
enum PresetCommands {
    List,
    Show { preset: String },
}

#[derive(Debug, Subcommand)]
enum RepoCommands {
    Status {
        service: String,
    },
    Pull {
        service: String,
    },
    Commit {
        service: String,
        #[arg(short, long)]
        message: String,
    },
    Push {
        service: String,
    },
}

#[derive(Debug, Subcommand)]
enum SecretCommands {
    List {
        service: String,
    },
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
    Remove {
        service: String,
        name: String,
    },
    Check {
        service: String,
    },
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
                    preset,
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
                preset,
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
        Commands::Preset { command } => preset_command(command),
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

    for service in &services {
        if let Some(preset) = service.preset.as_deref() {
            validate_preset(preset, &service.name)?;
        }
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
    preset: Option<String>,
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
    if let Some(preset) = input.preset.as_deref() {
        validate_preset(preset, &input.service)?;
    }

    let mut include = input.include;
    let mut exclude = input.exclude;
    normalize_values(&mut include);
    normalize_values(&mut exclude);

    let service = ServiceConfig {
        name: input.service,
        root: input.root,
        repo: input.repo,
        preset: input.preset,
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

fn preset_command(command: PresetCommands) -> Result<()> {
    match command {
        PresetCommands::List => {
            for name in preset_names() {
                println!("{name}");
            }
            Ok(())
        }
        PresetCommands::Show { preset } => {
            let preset =
                find_preset(&preset).with_context(|| format!("unknown preset {preset}"))?;
            println!("preset: {}", preset.name);
            println!("include:");
            for pattern in preset.include {
                println!("  {pattern}");
            }
            println!("exclude:");
            for pattern in preset.exclude {
                println!("  {pattern}");
            }
            Ok(())
        }
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
        "restore --dry-run <service>",
        "preset list",
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
        "preset list" => preset_command(PresetCommands::List),
        "status <service>"
        | "diff <service>"
        | "backup --dry-run <service>"
        | "restore --dry-run <service>" => {
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
                "restore --dry-run <service>" => restore(
                    paths,
                    &service,
                    true,
                    false,
                    false,
                    false,
                    PathSelection::default(),
                ),
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

    if let Some(name) = service.preset.as_deref()
        && let Some(preset) = find_preset(name)
    {
        include.extend(preset.include);
        exclude.extend(preset.exclude);
    }

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

fn validate_preset(preset: &str, service_name: &str) -> Result<()> {
    if find_preset(preset).is_none() {
        bail!("unknown preset {preset} for service {service_name}");
    }

    Ok(())
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
    if let Some(os) = service.conditions.os.as_deref()
        && os != std::env::consts::OS
    {
        return false;
    }
    if let Some(hostname) = service.conditions.hostname.as_deref()
        && hostname != current_hostname().as_deref().unwrap_or_default()
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
