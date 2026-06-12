use anyhow::Result;
use clap::Parser;
use lattice_core::paths::LatticePaths;

mod cli;
mod commands;
mod config_store;
mod output;
mod runtime;
mod service_state;

use cli::{Cli, Commands, ServiceCommands};
use commands::discover::discover;
use commands::service::{PatternTarget, ServiceAddInput};
use commands::sync::BackupCommandOptions;
use service_state::selection;

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
        Commands::Init { force } => commands::setup::init(&paths, force),
        Commands::Doctor => commands::setup::doctor(&paths),
        Commands::Validate => commands::setup::validate(&paths),
        Commands::Service {
            command: ServiceCommands::List,
        } => commands::service::list(&paths),
        Commands::Service {
            command: ServiceCommands::Show { service },
        } => commands::service::show(&paths, &service),
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
        } => commands::service::add(
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
        } => commands::service::remove(&paths, &service, yes),
        Commands::Include { command } => {
            commands::service::update_patterns(&paths, command, PatternTarget::Include)
        }
        Commands::Exclude { command } => {
            commands::service::update_patterns(&paths, command, PatternTarget::Exclude)
        }
        Commands::Permission { command } => commands::permission::run(&paths, command),
        Commands::App { command } => commands::app::run(&paths, command),
        Commands::Group { command } => commands::group::run(&paths, command),
        Commands::Bootstrap { command } => commands::bootstrap::run(&paths, command),
        Commands::Repo { command } => commands::repo::run(&paths, command),
        Commands::Secret { command } => commands::secret::run(&paths, command),
        Commands::Track {
            service,
            paths: items,
        } => commands::service::track(&paths, &service, items),
        Commands::Adopt {
            allow_secret_looking_files,
            allow_metadata_loss,
            service,
            paths: items,
        } => commands::sync::adopt(
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
        } => commands::sync::diff(&paths, &service, json, selection(only, exclude)),
        Commands::Tui { dry_run } => commands::tui::run(&paths, dry_run),
        Commands::Plan {
            json,
            only,
            exclude,
            service,
        } => commands::sync::plan(&paths, &service, json, selection(only, exclude)),
        Commands::Status {
            json,
            only,
            exclude,
            service,
        } => commands::sync::status(&paths, &service, json, selection(only, exclude)),
        Commands::Backup {
            dry_run,
            json,
            yes,
            allow_secret_looking_files,
            allow_metadata_loss,
            only,
            exclude,
            service,
        } => commands::sync::backup(
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
        } => commands::sync::restore(
            &paths,
            &service,
            dry_run,
            json,
            force,
            yes,
            selection(only, exclude),
        ),
        Commands::Snapshot { command } => commands::snapshot::run(&paths, command),
        Commands::Undo {
            dry_run,
            json,
            yes,
            snapshot,
            service,
        } => commands::snapshot::undo(&paths, &snapshot, service.as_deref(), dry_run, json, yes),
        Commands::Discover { json } => discover(&paths, json),
    }
}
