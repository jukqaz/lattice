use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lattice_core::app_catalog::find_app;
use lattice_core::paths::LatticePaths;
use lattice_core::secrets::find_secret_like_patterns;

pub(crate) fn discover(paths: &LatticePaths, json_output: bool) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?);
    let mut suggestions = Vec::new();
    suggestions.extend(discover_config_dirs(&home)?);
    if let Some(shell) = discover_shell(&home) {
        suggestions.push(shell);
    }
    suggestions.sort_by(|left, right| left.name.cmp(&right.name));

    let next_actions = discover_next_actions();

    if json_output {
        let values = suggestions
            .iter()
            .map(|suggestion| {
                serde_json::json!({
                    "name": suggestion.name,
                    "root": suggestion.root.display().to_string(),
                    "include": suggestion.include,
                    "exclude": suggestion.exclude,
                    "reason": suggestion.reason,
                    "warnings": suggestion.warnings,
                    "uses_app_catalog": find_app(&suggestion.name).is_some(),
                    "next_command": discover_next_command(suggestion)
                })
            })
            .collect::<Vec<_>>();
        print_json(serde_json::json!({
            "suggestions": values,
            "mutated": false,
            "services_dir": paths.services_dir.display().to_string(),
            "next_actions": next_actions
        }))?;
        return Ok(());
    }

    for suggestion in suggestions {
        println!("{} root={}", suggestion.name, suggestion.root.display());
        println!("  include: {}", suggestion.include.join(", "));
        if !suggestion.exclude.is_empty() {
            println!("  exclude: {}", suggestion.exclude.join(", "));
        }
        for warning in &suggestion.warnings {
            println!("  warning: {warning}");
        }
        println!("  next command: {}", discover_next_command(&suggestion));
    }
    println!("mutated: no");
    println!("next actions:");
    for action in next_actions {
        println!("- {action}");
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct DiscoverySuggestion {
    name: String,
    root: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    reason: String,
    warnings: Vec<String>,
}

fn discover_next_actions() -> Vec<&'static str> {
    vec![
        "review suggestions and choose one service to add",
        "run lattice plan <service> before backup or restore",
        "run lattice backup --dry-run <service> before writing repo files",
    ]
}

fn discover_next_command(suggestion: &DiscoverySuggestion) -> String {
    let root = shell_quote_if_needed(&suggestion.root.display().to_string());
    match suggestion.include.first() {
        Some(include) => format!(
            "lattice service add {} --root {root} --include {}",
            suggestion.name,
            shell_quote_if_needed(include)
        ),
        None => "review warnings before adding a service".to_string(),
    }
}

fn shell_quote_if_needed(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn discover_config_dirs(home: &Path) -> Result<Vec<DiscoverySuggestion>> {
    let config = home.join(".config");
    if !config.is_dir() {
        return Ok(Vec::new());
    }
    let mut suggestions = Vec::new();
    for entry in
        fs::read_dir(&config).with_context(|| format!("failed to read {}", config.display()))?
    {
        let entry = entry?;
        let root = entry.path();
        if !path_is_directory(&root)? {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lattice" || name.starts_with('.') {
            continue;
        }
        let (include, exclude, warnings) = conservative_patterns(&root)?;
        if include.is_empty() && warnings.is_empty() {
            continue;
        }
        let reason = if include.is_empty() {
            "local XDG config directory with only excluded warning-level files"
        } else {
            "local XDG config directory with non-secret-looking files"
        };
        suggestions.push(DiscoverySuggestion {
            name,
            root,
            include,
            exclude,
            reason: reason.to_string(),
            warnings,
        });
    }
    Ok(suggestions)
}
fn discover_shell(home: &Path) -> Option<DiscoverySuggestion> {
    let (mut include, mut exclude, warnings) = discover_shell_patterns(home);
    include.sort();
    exclude.extend([
        ".cache/**".to_string(),
        ".config/**".to_string(),
        ".local/share/**".to_string(),
        ".ssh/**".to_string(),
    ]);
    exclude.sort();
    exclude.dedup();
    if include.is_empty() && warnings.is_empty() {
        return None;
    }
    let reason = if include.is_empty() {
        "common shell startup files with only excluded warning-level files"
    } else {
        "common shell startup files"
    };
    Some(DiscoverySuggestion {
        name: "shell".to_string(),
        root: home.to_path_buf(),
        include,
        exclude,
        reason: reason.to_string(),
        warnings,
    })
}

fn discover_shell_patterns(home: &Path) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    let mut warnings = Vec::new();
    for name in [".bashrc", ".zshrc", ".profile"] {
        let path = home.join(name);
        let Ok(true) = discovery_file_is_small(&path) else {
            continue;
        };
        match discovery_secret_like_patterns(&path) {
            Ok(patterns) if patterns.is_empty() => include.push(name.to_string()),
            Ok(patterns) => {
                exclude.push(name.to_string());
                warnings.push(discovery_secret_warning(name, &patterns));
            }
            Err(_) => {}
        }
    }
    (include, exclude, warnings)
}

fn conservative_patterns(root: &Path) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    let mut warnings = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = fs::symlink_metadata(entry.path())
            .with_context(|| format!("failed to stat {}", entry.path().display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let is_dir = metadata.is_dir();
        if should_exclude_discovery_name(&name, is_dir) {
            exclude.push(if is_dir { format!("{name}/**") } else { name });
            continue;
        }
        if metadata.is_file() && discovery_file_is_small(&entry.path())? {
            let patterns = discovery_secret_like_patterns(&entry.path())?;
            if !patterns.is_empty() {
                exclude.push(name.clone());
                warnings.push(discovery_secret_warning(&name, &patterns));
                continue;
            }
            include.push(name);
        }
    }
    include.sort();
    include.dedup();
    exclude.sort();
    exclude.dedup();
    warnings.sort();
    warnings.dedup();
    Ok((include, exclude, warnings))
}

fn discovery_secret_like_patterns(path: &Path) -> Result<Vec<String>> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let content = String::from_utf8_lossy(&bytes);
    Ok(find_secret_like_patterns(&content))
}

fn discovery_secret_warning(path: &str, patterns: &[String]) -> String {
    format!(
        "excluded {path} because it contains secret-looking content ({})",
        patterns.join(", ")
    )
}

fn should_exclude_discovery_name(name: &str, is_dir: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("secret")
        || lower.contains("token")
        || lower.contains("auth")
        || lower.contains("credential")
        || lower.contains("session")
        || lower.contains("cache")
        || lower.ends_with(".db")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite3")
        || (is_dir && lower == "logs")
}

fn discovery_file_is_small(path: &Path) -> Result<bool> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    Ok(!metadata.file_type().is_symlink() && metadata.is_file() && metadata.len() <= 1024 * 1024)
}

fn path_is_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to stat {}", path.display())),
    }
}

fn print_json(value: serde_json::Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
