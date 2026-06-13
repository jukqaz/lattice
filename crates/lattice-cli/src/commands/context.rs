use anyhow::Result;
use lattice_core::paths::LatticePaths;

use crate::config_store::load_global_config;
use crate::output::print_json;
use crate::runtime::current_hostname;
use crate::service_state::normalize_labels_preserving_order;

pub(crate) fn show(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let global = load_global_config(paths)?;
    let mut contexts = global.contexts;
    normalize_labels_preserving_order(&mut contexts);
    let hostname = current_hostname();

    if json_output {
        print_json(serde_json::json!({
            "profile": global.profile,
            "contexts": contexts,
            "os": std::env::consts::OS,
            "hostname": hostname
        }))?;
        return Ok(());
    }

    println!("profile: {}", global.profile);
    println!("os: {}", std::env::consts::OS);
    println!("hostname: {}", hostname.as_deref().unwrap_or("unknown"));
    println!(
        "contexts: {}",
        if contexts.is_empty() {
            "none".to_string()
        } else {
            contexts.join(", ")
        }
    );
    Ok(())
}
