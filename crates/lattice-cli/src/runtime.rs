use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

pub(crate) fn git_remote_status(repo: &Path) -> String {
    if !repo.join(".git").exists() {
        return "missing".to_string();
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", "origin"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "missing".to_string(),
    }
}

pub(crate) fn git_dirty(repo: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["status", "--porcelain"])
        .output();
    output
        .map(|output| output.status.success() && !output.stdout.is_empty())
        .unwrap_or(false)
}

pub(crate) fn expand_path(path: &str) -> Result<PathBuf> {
    if path == "~" {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        return Ok(PathBuf::from(home));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var_os("HOME").context("HOME is not set")?;
        return Ok(PathBuf::from(home).join(rest));
    }

    Ok(PathBuf::from(path))
}

pub(crate) fn availability(bin: &str) -> &'static str {
    let available = which::which(bin).is_ok()
        && Command::new(bin)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

    if available { "available" } else { "missing" }
}

pub(crate) fn current_hostname() -> Option<String> {
    std::env::var("HOSTNAME").ok().or_else(|| {
        Command::new("hostname").output().ok().and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
    })
}
