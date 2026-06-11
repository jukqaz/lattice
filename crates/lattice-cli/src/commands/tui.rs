use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use inquire::Select;
use lattice_core::ops::PathSelection;
use lattice_core::paths::LatticePaths;
use lattice_core::scanner::scan_service;

use crate::{
    BackupCommandOptions, backup, cli::AppCommands, diff, effective_patterns, expand_path,
    load_services, plan, resolve_repo_path, service_is_active, service_list, status, validate,
};

pub(crate) fn run(paths: &LatticePaths, dry_run: bool) -> Result<()> {
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
        "app list" => crate::commands::app::run(paths, AppCommands::List),
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
