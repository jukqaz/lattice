use anyhow::Result;
use lattice_core::config::SecretRef;
use lattice_core::paths::LatticePaths;

use crate::cli::SecretCommands;
use crate::config_store::{load_service, write_service_config};
use crate::service_state::{
    secret_backend_status, secret_item_for_backend, validate_secret_backend,
};

pub(crate) fn run(paths: &LatticePaths, command: SecretCommands) -> Result<()> {
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
            let item = secret_item_for_backend(&backend, item, env.as_deref())?;
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
                let status = secret_backend_status(&secret);
                println!(
                    "{} backend={} status={} item={} env={} value=not-read",
                    secret.name,
                    secret.backend,
                    status,
                    secret.item,
                    secret.env.as_deref().unwrap_or("-")
                );
            }
            Ok(())
        }
    }
}
