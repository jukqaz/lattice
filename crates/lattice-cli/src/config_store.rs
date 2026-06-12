use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use lattice_core::config::{GlobalConfig, ServiceConfig};
use lattice_core::paths::LatticePaths;

use crate::service_state::default_repo_dir_name;

pub(crate) fn write_file_if_allowed(path: &Path, body: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Ok(());
    }
    write_private_file(path, body)
}

pub(crate) fn write_private_file(path: &Path, body: &str) -> Result<()> {
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

pub(crate) fn load_service(paths: &LatticePaths, service_name: &str) -> Result<ServiceConfig> {
    let path = service_file_path(paths, service_name)?;
    let body =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let service: ServiceConfig =
        toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))?;
    ensure_service_config_name_matches(&path, service_name, &service)?;
    Ok(service)
}

pub(crate) fn write_service_config(paths: &LatticePaths, service: &ServiceConfig) -> Result<()> {
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

pub(crate) fn service_file_path(paths: &LatticePaths, service_name: &str) -> Result<PathBuf> {
    Ok(paths
        .services_dir
        .join(format!("{}.toml", default_repo_dir_name(service_name)?)))
}

pub(crate) fn load_global_config(paths: &LatticePaths) -> Result<GlobalConfig> {
    let body = fs::read_to_string(&paths.config_file)
        .with_context(|| format!("failed to read {}", paths.config_file.display()))?;
    toml::from_str(&body)
        .with_context(|| format!("failed to parse {}", paths.config_file.display()))
}

pub(crate) fn load_services(paths: &LatticePaths) -> Result<Vec<ServiceConfig>> {
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

pub(crate) fn ensure_service_config_name_matches(
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
