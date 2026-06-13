use anyhow::Result;
use lattice_core::config::ServiceConfig;
use lattice_core::paths::LatticePaths;

use crate::cli::BootstrapCommands;

use crate::commands::group::service_root_exists;
use crate::config_store::load_services;
use crate::output::{available_missing, present_missing, print_json, yes_no};
use crate::runtime::{expand_path, git_dirty, git_remote_status};
use crate::service_state::{resolve_repo_path, service_is_active};

#[derive(Debug)]
struct BootstrapServiceReport {
    service: String,
    active: bool,
    root: String,
    root_exists: bool,
    repo: String,
    repo_exists: bool,
    git_repo: bool,
    remote: String,
    dirty: bool,
    manifest: &'static str,
    issues: Vec<String>,
    warnings: Vec<String>,
    ready: bool,
}

impl BootstrapServiceReport {
    fn inspect(paths: &LatticePaths, service: ServiceConfig, git_available: bool) -> Result<Self> {
        let root = expand_path(&service.root)?;
        let repo = resolve_repo_path(paths, &service)?;
        let manifest = repo.join(".lattice").join("manifest.toml");
        let active = service_is_active(paths, &service)?;
        let root_exists = service_root_exists(&root)?;
        let repo_exists = repo.exists();
        let git_repo = repo.join(".git").exists();
        let remote = if git_repo && git_available {
            git_remote_status(&repo)
        } else {
            "missing".to_string()
        };
        let dirty = git_repo && git_available && git_dirty(&repo);
        let manifest_exists = manifest.exists();
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
        if !manifest_exists {
            issues.push("missing_manifest".to_string());
        }

        Ok(Self {
            service: service.name,
            active,
            root: root.display().to_string(),
            root_exists,
            repo: repo.display().to_string(),
            repo_exists,
            git_repo,
            remote,
            dirty,
            manifest: present_missing(manifest_exists),
            ready: active && root_exists && manifest_exists && issues.is_empty(),
            issues,
            warnings,
        })
    }

    fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "service": self.service,
            "active": self.active,
            "root": self.root,
            "root_exists": self.root_exists,
            "repo": self.repo,
            "repo_exists": self.repo_exists,
            "git_repo": self.git_repo,
            "remote": self.remote,
            "dirty": self.dirty,
            "manifest": self.manifest,
            "issues": self.issues,
            "warnings": self.warnings,
            "ready": self.ready
        })
    }

    fn print_human(&self) {
        println!(
            "- {} active={} root={} repo={} git={} manifest={} ready={} issues={} warnings={}",
            self.service,
            yes_no(self.active),
            present_missing(self.root_exists),
            present_missing(self.repo_exists),
            yes_no(self.git_repo),
            self.manifest,
            yes_no(self.ready),
            self.issues.len(),
            self.warnings.len()
        );
    }
}

pub(crate) fn run(paths: &LatticePaths, command: BootstrapCommands) -> Result<()> {
    match command {
        BootstrapCommands::Check { json } => bootstrap_check(paths, json),
    }
}

pub(crate) fn bootstrap_check(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let config_exists = paths.config_file.exists();
    let services_dir_exists = paths.services_dir.is_dir();
    let git_available = which::which("git").is_ok();
    let services = if services_dir_exists {
        load_services(paths)?
    } else {
        Vec::new()
    };
    let service_reports = services
        .into_iter()
        .map(|service| BootstrapServiceReport::inspect(paths, service, git_available))
        .collect::<Result<Vec<_>>>()?;
    let ready_count = service_reports.iter().filter(|report| report.ready).count();
    let any_service_issues = service_reports
        .iter()
        .any(BootstrapServiceReport::has_issues);
    let any_service_warnings = service_reports
        .iter()
        .any(BootstrapServiceReport::has_warnings);
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
                "git": available_missing(git_available)
            },
            "git": available_missing(git_available),
            "services": service_reports.iter().map(BootstrapServiceReport::json).collect::<Vec<_>>(),
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
        present_missing(config_exists)
    );
    println!(
        "services: {} ({})",
        paths.services_dir.display(),
        present_missing(services_dir_exists)
    );
    println!("git: {}", available_missing(git_available));
    println!("ready services: {ready_count}");
    for report in &service_reports {
        report.print_human();
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
