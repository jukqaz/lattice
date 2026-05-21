use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::config::{HookConfig, HooksConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPhase {
    BeforeBackup,
    AfterBackup,
    BeforeRestore,
    AfterRestore,
}

impl HookPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::BeforeBackup => "before_backup",
            Self::AfterBackup => "after_backup",
            Self::BeforeRestore => "before_restore",
            Self::AfterRestore => "after_restore",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookStatus {
    WouldRun,
    Ran,
    SkippedConfirm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookOutcome {
    pub phase: HookPhase,
    pub name: String,
    pub status: HookStatus,
}

pub fn hooks_for_phase(hooks: &HooksConfig, phase: HookPhase) -> &[HookConfig] {
    match phase {
        HookPhase::BeforeBackup => &hooks.before_backup,
        HookPhase::AfterBackup => &hooks.after_backup,
        HookPhase::BeforeRestore => &hooks.before_restore,
        HookPhase::AfterRestore => &hooks.after_restore,
    }
}

pub fn run_hooks(
    hooks: &HooksConfig,
    phase: HookPhase,
    dry_run: bool,
    yes: bool,
) -> Result<Vec<HookOutcome>> {
    let mut outcomes = Vec::new();

    for hook in hooks_for_phase(hooks, phase) {
        if dry_run {
            outcomes.push(HookOutcome {
                phase,
                name: hook.name.clone(),
                status: HookStatus::WouldRun,
            });
            continue;
        }

        if hook.confirm && !yes {
            outcomes.push(HookOutcome {
                phase,
                name: hook.name.clone(),
                status: HookStatus::SkippedConfirm,
            });
            continue;
        }

        let status = Command::new(&hook.command)
            .args(&hook.args)
            .status()
            .with_context(|| format!("failed to run hook {}", hook.name))?;

        if !status.success() {
            bail!("hook {} exited with {}", hook.name, status);
        }

        outcomes.push(HookOutcome {
            phase,
            name: hook.name.clone(),
            status: HookStatus::Ran,
        });
    }

    Ok(outcomes)
}
