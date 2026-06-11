use anyhow::Result;
use lattice_core::config::PermissionRule;
use lattice_core::paths::LatticePaths;

use crate::{
    cli::PermissionCommands, load_service, validate_mode, validate_relative_config_path,
    write_service_config,
};

pub(crate) fn run(paths: &LatticePaths, command: PermissionCommands) -> Result<()> {
    match command {
        PermissionCommands::Set {
            service,
            path,
            mode,
        } => {
            validate_mode(&mode)?;
            validate_relative_config_path(&path)?;
            let mut config = load_service(paths, &service)?;
            if let Some(rule) = config.permissions.iter_mut().find(|rule| rule.path == path) {
                rule.mode = mode;
            } else {
                config.permissions.push(PermissionRule { path, mode });
            }
            config
                .permissions
                .sort_by(|left, right| left.path.cmp(&right.path));
            write_service_config(paths, &config)?;
            println!("updated permission rules");
            Ok(())
        }
        PermissionCommands::Remove { service, path } => {
            let mut config = load_service(paths, &service)?;
            config.permissions.retain(|rule| rule.path != path);
            write_service_config(paths, &config)?;
            println!("removed permission rule");
            Ok(())
        }
    }
}
