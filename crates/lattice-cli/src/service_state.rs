use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use lattice_core::config::{SecretRef, ServiceConfig};
use lattice_core::ops::PathSelection;
use lattice_core::paths::LatticePaths;

use crate::runtime::{current_hostname, expand_path};

pub(crate) fn selection(only: Vec<String>, exclude: Vec<String>) -> PathSelection {
    PathSelection { only, exclude }
}

pub(crate) fn effective_patterns(service: &ServiceConfig) -> (Vec<String>, Vec<String>) {
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

pub(crate) fn resolve_repo_path(paths: &LatticePaths, service: &ServiceConfig) -> Result<PathBuf> {
    match service.repo.as_deref() {
        Some(repo) => expand_path(repo),
        None => Ok(paths
            .repo_cache_dir
            .join(default_repo_dir_name(&service.name)?)),
    }
}

pub(crate) fn default_repo_dir_name(service_name: &str) -> Result<&str> {
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

pub(crate) fn validate_secret_backend(backend: &str) -> Result<()> {
    if !matches!(backend, "rbw" | "bw" | "env") {
        bail!("secret backend must be rbw, bw, or env");
    }

    Ok(())
}

pub(crate) fn secret_item_for_backend(
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

pub(crate) fn secret_backend_status(secret: &SecretRef) -> &'static str {
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

pub(crate) fn ensure_service_active(service: &ServiceConfig) -> Result<()> {
    if !service_is_active(service) {
        bail!("service {} is inactive on this host", service.name);
    }

    Ok(())
}

pub(crate) fn service_is_active(service: &ServiceConfig) -> bool {
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

pub(crate) fn validate_mode(mode: &str) -> Result<()> {
    let parsed = u32::from_str_radix(mode, 8).with_context(|| format!("invalid mode {mode}"))?;
    if mode.len() != 4 || parsed > 0o777 {
        bail!("mode must be a four-digit file permission such as 0600 or 0755");
    }

    Ok(())
}

pub(crate) fn validate_relative_config_path(path: &str) -> Result<()> {
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

pub(crate) fn normalize_values(values: &mut Vec<String>) {
    values.retain(|value| !value.is_empty());
    values.sort();
    values.dedup();
}

pub(crate) fn snapshot_policy(requires_force: bool) -> &'static str {
    if requires_force {
        "forced restore snapshots conflicts before overwrite"
    } else {
        "no snapshot needed for non-conflicting restore"
    }
}
