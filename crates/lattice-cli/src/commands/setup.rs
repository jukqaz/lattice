use std::fs;

use anyhow::{Context, Result};
use lattice_core::paths::LatticePaths;

use crate::config_store::{load_global_config, load_services, write_file_if_allowed};
use crate::runtime::availability;
use crate::service_state::resolve_repo_path;

pub(crate) fn init(paths: &LatticePaths, force: bool) -> Result<()> {
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

pub(crate) fn doctor(paths: &LatticePaths) -> Result<()> {
    println!("config: {}", paths.config_file.display());
    println!("services: {}", paths.services_dir.display());
    println!("repos: {}", paths.repo_cache_dir.display());
    println!("state: {}", paths.state_dir.display());
    println!("cache: {}", paths.cache_dir.display());
    println!("rbw: {}", availability("rbw"));
    println!("bw: {}", availability("bw"));
    Ok(())
}

pub(crate) fn validate(paths: &LatticePaths) -> Result<()> {
    let global = load_global_config(paths)?;
    let services = load_services(paths)?;
    crate::commands::group::validate_groups(&global.groups, &services)?;

    for service in &services {
        let _ = resolve_repo_path(paths, service)?;
    }

    println!("valid config");
    println!("profile: {}", global.profile);
    println!("services: {}", services.len());
    Ok(())
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
