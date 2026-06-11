use std::path::PathBuf;

use anyhow::Result;
use lattice_core::hooks::{HookOutcome, HookStatus};
use lattice_core::manifest::ManifestEntry;

pub(crate) fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub(crate) fn present_missing(value: bool) -> &'static str {
    if value { "present" } else { "missing" }
}

pub(crate) fn available_missing(value: bool) -> &'static str {
    if value { "available" } else { "missing" }
}

pub(crate) fn path_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

pub(crate) fn manifest_entry_strings(entries: &[ManifestEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry.path.display().to_string())
        .collect()
}

pub(crate) fn hook_outcomes_json(
    before: &[HookOutcome],
    after: &[HookOutcome],
) -> Vec<serde_json::Value> {
    before
        .iter()
        .chain(after.iter())
        .map(|outcome| {
            let status = match outcome.status {
                HookStatus::WouldRun => "would_run",
                HookStatus::Ran => "ran",
                HookStatus::SkippedConfirm => "skipped_confirm",
            };
            serde_json::json!({
                "phase": outcome.phase.label(),
                "name": outcome.name,
                "status": status
            })
        })
        .collect()
}

pub(crate) fn print_json(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn print_hook_outcomes(outcomes: &[HookOutcome]) {
    for outcome in outcomes {
        match outcome.status {
            HookStatus::WouldRun => {
                println!("would run hook {}: {}", outcome.phase.label(), outcome.name);
            }
            HookStatus::Ran => {
                println!("ran hook {}: {}", outcome.phase.label(), outcome.name);
            }
            HookStatus::SkippedConfirm => {
                println!(
                    "skipped hook {}: {} (requires --yes)",
                    outcome.phase.label(),
                    outcome.name
                );
            }
        }
    }
}
