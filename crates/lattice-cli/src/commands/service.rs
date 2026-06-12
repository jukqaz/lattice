use std::fs;

use anyhow::{Context, Result, bail};
use lattice_core::config::{ConditionsConfig, HooksConfig, RestoreConfig, ServiceConfig};
use lattice_core::paths::LatticePaths;

use crate::cli::PatternCommands;
use crate::config_store::{load_service, load_services, service_file_path, write_service_config};
use crate::service_state::normalize_values;

#[derive(Debug, Clone)]
pub(crate) struct ServiceAddInput {
    pub(crate) service: String,
    pub(crate) root: String,
    pub(crate) repo: Option<String>,
    pub(crate) include: Vec<String>,
    pub(crate) exclude: Vec<String>,
    pub(crate) template: bool,
    pub(crate) symlink: bool,
    pub(crate) os: Option<String>,
    pub(crate) hostname: Option<String>,
    pub(crate) force: bool,
}

pub(crate) fn list(paths: &LatticePaths) -> Result<()> {
    for service in load_services(paths)? {
        println!("{}", service.name);
    }
    Ok(())
}

pub(crate) fn show(paths: &LatticePaths, service_name: &str) -> Result<()> {
    let path = service_file_path(paths, service_name)?;
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    print!("{body}");
    Ok(())
}

pub(crate) fn add(paths: &LatticePaths, input: ServiceAddInput) -> Result<()> {
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

pub(crate) fn remove(paths: &LatticePaths, service_name: &str, yes: bool) -> Result<()> {
    let path = service_file_path(paths, service_name)?;
    if !yes {
        bail!("removing service {service_name} requires --yes");
    }
    fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    println!("removed service {service_name}");
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PatternTarget {
    Include,
    Exclude,
}

pub(crate) fn update_patterns(
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

pub(crate) fn track(paths: &LatticePaths, service_name: &str, items: Vec<String>) -> Result<()> {
    let mut service = load_service(paths, service_name)?;
    service.include.extend(items);
    normalize_values(&mut service.include);
    write_service_config(paths, &service)?;
    println!("tracked {} include patterns", service.include.len());
    Ok(())
}

fn pattern_target_label(target: PatternTarget) -> &'static str {
    match target {
        PatternTarget::Include => "include",
        PatternTarget::Exclude => "exclude",
    }
}
