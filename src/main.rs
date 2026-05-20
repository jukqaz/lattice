use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use lattice::config::ServiceConfig;
use lattice::manifest::read_manifest;
use lattice::ops::{apply_permission_rules, backup_service, create_restore_dirs, restore_service};
use lattice::paths::LatticePaths;
use lattice::preset::codex_preset;
use lattice::scanner::scan_service;

#[derive(Debug, Parser)]
#[command(name = "lattice")]
#[command(about = "A small dotfiles backup and restore manager")]
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
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
    Status {
        service: String,
    },
    Backup {
        #[arg(long)]
        dry_run: bool,
        service: String,
    },
    Restore {
        #[arg(long)]
        dry_run: bool,
        service: String,
    },
}

#[derive(Debug, Subcommand)]
enum ServiceCommands {
    List,
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
        Commands::Service {
            command: ServiceCommands::List,
        } => service_list(&paths),
        Commands::Status { service } => status(&paths, &service),
        Commands::Backup { dry_run, service } => backup(&paths, &service, dry_run),
        Commands::Restore { dry_run, service } => restore(&paths, &service, dry_run),
    }
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

    let codex_service = format!(
        r#"name = "codex"
root = "~/.codex"
repo = "{}"
preset = "codex"

[restore]
create_dirs = [
  {{ path = "shell_snapshots", mode = "0700" }},
  {{ path = "bin", mode = "0755" }},
]

[[permissions]]
path = "config.toml"
mode = "0600"

[[permissions]]
path = "bin/mcp-rbw"
mode = "0700"
"#,
        paths.repo_cache_dir.join("codex").display()
    );
    write_file_if_allowed(
        &paths.services_dir.join("codex.toml"),
        &codex_service,
        force,
    )?;

    println!("initialized {}", config_dir.display());
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

fn service_list(paths: &LatticePaths) -> Result<()> {
    for service in load_services(paths)? {
        println!("{}", service.name);
    }
    Ok(())
}

fn status(paths: &LatticePaths, service_name: &str) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = expand_path(&service.repo)?;
    let files = scan_service(&root, &include, &exclude)?;
    let manifest = repo.join(".lattice").join("manifest.toml");

    println!("service: {}", service.name);
    println!("root: {}", root.display());
    println!("repo: {}", repo.display());
    println!("included files: {}", files.len());
    println!(
        "manifest: {}",
        if manifest.exists() {
            "present"
        } else {
            "missing"
        }
    );
    Ok(())
}

fn backup(paths: &LatticePaths, service_name: &str, dry_run: bool) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let (include, exclude) = effective_patterns(&service);
    let root = expand_path(&service.root)?;
    let repo = expand_path(&service.repo)?;

    if dry_run {
        let files = scan_service(&root, &include, &exclude)?;
        println!("would copy {} files to {}", files.len(), repo.display());
        for file in files {
            println!("{}", file.display());
        }
        return Ok(());
    }

    let report = backup_service(&root, &repo, &include, &exclude)?;

    println!("copied {} files to {}", report.copied.len(), repo.display());
    println!("manifest: {}", report.manifest_path.display());
    Ok(())
}

fn restore(paths: &LatticePaths, service_name: &str, dry_run: bool) -> Result<()> {
    let service = load_service(paths, service_name)?;
    let root = expand_path(&service.root)?;
    let repo = expand_path(&service.repo)?;

    if dry_run {
        let manifest = read_manifest(&repo.join(".lattice").join("manifest.toml"))?;
        println!(
            "would restore {} files to {}",
            manifest.entries.len(),
            root.display()
        );
        for entry in manifest.entries {
            println!("{}", entry.path.display());
        }
        return Ok(());
    }

    let report = restore_service(&repo, &root)?;
    let created_dirs = create_restore_dirs(&root, &service.restore.create_dirs)?;
    let applied_permissions = apply_permission_rules(&root, &service.permissions)?;

    println!(
        "restored {} files to {}",
        report.restored.len(),
        root.display()
    );
    if !created_dirs.is_empty() {
        println!("created {} restore dirs", created_dirs.len());
    }
    if !applied_permissions.is_empty() {
        println!("applied {} permission rules", applied_permissions.len());
    }
    Ok(())
}

fn write_file_if_allowed(path: &Path, body: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))
}

fn load_service(paths: &LatticePaths, service_name: &str) -> Result<ServiceConfig> {
    let path = paths.services_dir.join(format!("{service_name}.toml"));
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))
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
        services.push(
            toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))?,
        );
    }

    services.sort_by(|left: &ServiceConfig, right| left.name.cmp(&right.name));
    Ok(services)
}

fn effective_patterns(service: &ServiceConfig) -> (Vec<String>, Vec<String>) {
    let mut include = Vec::new();
    let mut exclude = Vec::new();

    if service.preset.as_deref() == Some("codex") {
        let preset = codex_preset();
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
    let available = Command::new(bin)
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
