use anyhow::{Context, Result};
use lattice_core::app_catalog::{app_names, find_app};
use lattice_core::paths::LatticePaths;

use crate::cli::AppCommands;
use crate::commands::service::{ServiceAddInput, add as service_add};

pub(crate) fn run(paths: &LatticePaths, command: AppCommands) -> Result<()> {
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
            contexts,
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
                    contexts,
                    force,
                },
            )
        }
    }
}
