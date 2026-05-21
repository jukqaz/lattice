use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

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

        run_hook(hook)?;

        outcomes.push(HookOutcome {
            phase,
            name: hook.name.clone(),
            status: HookStatus::Ran,
        });
    }

    Ok(outcomes)
}

fn run_hook(hook: &HookConfig) -> Result<()> {
    let mut child = Command::new(&hook.command)
        .args(&hook.args)
        .spawn()
        .with_context(|| format!("failed to run hook {}", hook.name))?;

    let status = if let Some(timeout_sec) = hook.timeout_sec {
        wait_with_timeout(&mut child, &hook.name, timeout_sec)?
    } else {
        child
            .wait()
            .with_context(|| format!("failed to wait for hook {}", hook.name))?
    };

    ensure_hook_success(&hook.name, status)
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    hook_name: &str,
    timeout_sec: u64,
) -> Result<ExitStatus> {
    let timeout = Duration::from_secs(timeout_sec);
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("failed to wait for hook {hook_name}"))?
        {
            return Ok(status);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!("hook {hook_name} timed out after {timeout_sec}s");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn ensure_hook_success(hook_name: &str, status: ExitStatus) -> Result<()> {
    if !status.success() {
        bail!("hook {hook_name} exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::config::{HookConfig, HooksConfig};

    use super::{HookPhase, run_hooks};

    #[test]
    fn run_hooks_enforces_timeout_sec() {
        let hooks = HooksConfig {
            before_backup: vec![HookConfig {
                name: "slow hook".to_string(),
                command: "/bin/sleep".to_string(),
                args: vec!["3".to_string()],
                timeout_sec: Some(1),
                confirm: false,
            }],
            ..HooksConfig::default()
        };

        let started = Instant::now();
        let error = run_hooks(&hooks, HookPhase::BeforeBackup, false, false)
            .expect_err("slow hook should time out");

        assert!(format!("{error:#}").contains("timed out"));
        assert!(started.elapsed().as_secs() < 3);
    }
}
