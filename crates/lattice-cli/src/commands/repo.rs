use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use lattice_core::paths::LatticePaths;
use lattice_core::secrets::find_secret_like_patterns;

use crate::{cli::RepoCommands, load_service, resolve_repo_path};

pub(crate) fn run(paths: &LatticePaths, command: RepoCommands) -> Result<()> {
    match command {
        RepoCommands::Status { service } => {
            run_repo_command(paths, &service, ["status", "--short", "--branch"])
        }
        RepoCommands::Pull { service } => run_repo_command(paths, &service, ["pull", "--ff-only"]),
        RepoCommands::Commit { service, message } => {
            let repo = repo_for_command(paths, &service)?;
            ensure_repo_has_no_secret_like_content(&repo)?;
            run_git_passthrough(&repo, ["add", "."])?;
            run_git_passthrough(&repo, ["commit", "-m", &message])
        }
        RepoCommands::Push { service } => {
            let repo = repo_for_command(paths, &service)?;
            ensure_repo_has_no_secret_like_content(&repo)?;
            run_git_passthrough(&repo, ["push"])
        }
    }
}

fn run_repo_command<const N: usize>(
    paths: &LatticePaths,
    service_name: &str,
    args: [&str; N],
) -> Result<()> {
    let repo = repo_for_command(paths, service_name)?;
    run_git_passthrough(&repo, args)
}

fn repo_for_command(paths: &LatticePaths, service_name: &str) -> Result<PathBuf> {
    let service = load_service(paths, service_name)?;
    let repo = resolve_repo_path(paths, &service)?;
    if !repo.exists() {
        bail!("repo does not exist: {}", repo.display());
    }
    if !repo.join(".git").exists() {
        bail!("repo is not a git repository: {}", repo.display());
    }
    Ok(repo)
}

fn run_git_passthrough<const N: usize>(repo: &Path, args: [&str; N]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .with_context(|| format!("failed to run git in {}", repo.display()))?;
    if !status.success() {
        bail!("git exited with {status}");
    }
    Ok(())
}

fn ensure_repo_has_no_secret_like_content(repo: &Path) -> Result<()> {
    let mut findings = Vec::new();
    collect_repo_secret_findings(repo, repo, &mut findings)?;
    if !findings.is_empty() {
        bail!(
            "secret-looking content found in repo: {}",
            findings.join("; ")
        );
    }
    Ok(())
}

fn collect_repo_secret_findings(root: &Path, dir: &Path, findings: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "repo contains symlink; review before commit: {}",
                path.display()
            );
        }
        if metadata.is_dir() {
            collect_repo_secret_findings(root, &path, findings)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        let content = String::from_utf8_lossy(&bytes);
        let patterns = find_secret_like_patterns(&content);
        if !patterns.is_empty() {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            findings.push(format!("{} ({})", relative.display(), patterns.join(", ")));
        }
    }
    Ok(())
}
