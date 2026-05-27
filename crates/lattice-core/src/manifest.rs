use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<ManifestEntry>,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestEntry {
    pub path: PathBuf,
    pub mode: String,
}

pub fn write_manifest(path: &Path, manifest: &Manifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let body = toml::to_string_pretty(manifest).context("failed to serialize manifest")?;
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))
}

pub fn read_manifest(path: &Path) -> Result<Manifest> {
    let body =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest =
        toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest) -> Result<()> {
    if manifest.version != 1 {
        bail!("unsupported manifest version: {}", manifest.version);
    }

    for entry in manifest.directories.iter().chain(manifest.entries.iter()) {
        ensure_safe_relative_path(&entry.path)?;
        validate_manifest_mode(&entry.mode)?;
    }
    ensure_no_portable_path_collisions(
        manifest
            .directories
            .iter()
            .chain(manifest.entries.iter())
            .map(|entry| entry.path.as_path()),
    )?;

    Ok(())
}

fn validate_manifest_mode(mode: &str) -> Result<()> {
    let parsed = if mode.len() == 4 && mode.chars().all(|ch| matches!(ch, '0'..='7')) {
        u32::from_str_radix(mode, 8).context("validated octal manifest mode should parse")?
    } else {
        bail!("invalid manifest mode: {mode}");
    };
    if parsed > 0o777 {
        bail!("invalid manifest mode: {mode}");
    }
    Ok(())
}

fn ensure_safe_relative_path(path: &Path) -> Result<()> {
    let mut has_component = false;
    for component in path.components() {
        match component {
            Component::Normal(component) => {
                has_component = true;
                ensure_portable_component(path, component)?;
            }
            _ => bail!("unsafe relative path: {}", path.display()),
        }
    }
    if !has_component {
        bail!("unsafe relative path: {}", path.display());
    }
    Ok(())
}

fn ensure_portable_component(path: &Path, component: &OsStr) -> Result<()> {
    let text = component
        .to_str()
        .with_context(|| format!("path is not portable UTF-8: {}", path.display()))?;
    if text.chars().any(char::is_control) {
        bail!(
            "path is not portable because it contains control characters: {}",
            path.display()
        );
    }
    Ok(())
}

fn ensure_no_portable_path_collisions<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Result<()> {
    let mut seen = HashMap::<String, PathBuf>::new();
    for path in paths {
        let key = portable_path_key(path)?;
        if let Some(existing) = seen.insert(key, path.to_path_buf()) {
            bail!(
                "portable path collision: {} conflicts with {}",
                existing.display(),
                path.display()
            );
        }
    }
    Ok(())
}

fn portable_path_key(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    let mut has_component = false;
    for component in path.components() {
        match component {
            Component::Normal(component) => {
                has_component = true;
                ensure_portable_component(path, component)?;
                let text = component
                    .to_str()
                    .with_context(|| format!("path is not portable UTF-8: {}", path.display()))?;
                let normalized = text
                    .chars()
                    .nfc()
                    .flat_map(char::to_lowercase)
                    .collect::<String>();
                parts.push(normalized);
            }
            _ => bail!("unsafe relative path: {}", path.display()),
        }
    }
    if !has_component {
        bail!("unsafe relative path: {}", path.display());
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::{Manifest, ManifestEntry, read_manifest, write_manifest};

    #[test]
    fn writes_and_reads_permission_manifest() {
        let temp = tempdir().expect("tempdir");
        let manifest_path = temp.path().join(".lattice/manifest.toml");
        let manifest = Manifest {
            version: 1,
            directories: vec![ManifestEntry {
                path: PathBuf::from("skills/empty-skill"),
                mode: "0755".to_string(),
            }],
            entries: vec![
                ManifestEntry {
                    path: PathBuf::from("config.toml"),
                    mode: "0600".to_string(),
                },
                ManifestEntry {
                    path: PathBuf::from("bin/mcp-rbw"),
                    mode: "0700".to_string(),
                },
            ],
        };

        write_manifest(&manifest_path, &manifest).expect("write manifest");
        let parsed = read_manifest(&manifest_path).expect("read manifest");

        assert_eq!(parsed, manifest);
    }

    #[test]
    fn rejects_negative_manifest_fixture_corpus() {
        let fixtures = [
            (
                "unknown top-level field",
                r#"
version = 1
entries = []
owner = "root"
"#,
                "unknown field",
            ),
            (
                "unknown entry field",
                r#"
version = 1
entries = [{ path = "config.toml", mode = "0600", checksum = "abc" }]
"#,
                "unknown field",
            ),
            (
                "unsupported version",
                r#"
version = 2
entries = []
"#,
                "unsupported manifest version",
            ),
            (
                "absolute path",
                r#"
version = 1
entries = [{ path = "/etc/passwd", mode = "0600" }]
"#,
                "unsafe relative path",
            ),
            (
                "parent traversal path",
                r#"
version = 1
entries = [{ path = "../escape.toml", mode = "0600" }]
"#,
                "unsafe relative path",
            ),
            (
                "current directory path",
                r#"
version = 1
entries = [{ path = ".", mode = "0600" }]
"#,
                "unsafe relative path",
            ),
            (
                "control character path",
                "version = 1\nentries = [{ path = \"bad\\nname.toml\", mode = \"0600\" }]\n",
                "path is not portable",
            ),
            (
                "non-octal mode",
                r#"
version = 1
entries = [{ path = "config.toml", mode = "06g0" }]
"#,
                "invalid manifest mode",
            ),
            (
                "signed mode",
                r#"
version = 1
entries = [{ path = "config.toml", mode = "+600" }]
"#,
                "invalid manifest mode",
            ),
            (
                "mode without leading zero",
                r#"
version = 1
entries = [{ path = "config.toml", mode = "600" }]
"#,
                "invalid manifest mode",
            ),
            (
                "mode with sticky bit",
                r#"
version = 1
entries = [{ path = "config.toml", mode = "1755" }]
"#,
                "invalid manifest mode",
            ),
            (
                "portable collision across sections",
                r#"
version = 1
directories = [{ path = "Config", mode = "0700" }]
entries = [{ path = "config", mode = "0600" }]
"#,
                "portable path collision",
            ),
        ];

        for (name, body, expected) in fixtures {
            let temp = tempdir().expect("tempdir");
            let manifest_path = temp.path().join("manifest.toml");
            fs::write(&manifest_path, body).unwrap_or_else(|error| panic!("write {name}: {error}"));

            let error = read_manifest(&manifest_path)
                .unwrap_err_or_else(|| panic!("fixture should be rejected: {name}"));

            assert!(
                format!("{error:#}").contains(expected),
                "fixture {name} should contain {expected:?}, got {error:#}"
            );
        }
    }

    #[test]
    fn read_manifest_handles_mutated_fixture_corpus_without_panicking() {
        let corpus: &[&[u8]] = &[
            b"",
            b"version = 1\nentries = []\n",
            b"version = 'one'\nentries = []\n",
            b"version = 1\nentries = [{ path = [], mode = {} }]\n",
            b"version = 1\nentries = [{ path = '../../x', mode = '0600' }]\n",
            b"version = 1\nentries = [{ path = 'config.toml', mode = '999999999999999999999' }]\n",
            b"version = 1\nentries = [{ path = 'a', mode = '0600' }, { path = 'A', mode = '0600' }]\n",
            b"version = 1\ndirectories = [{ path = 'e\xcc\x81', mode = '0700' }]\nentries = [{ path = '\xc3\xa9', mode = '0600' }]\n",
            b"\x80\x81\x82",
        ];

        for (index, body) in corpus.iter().enumerate() {
            let temp = tempdir().expect("tempdir");
            let manifest_path = temp.path().join(format!("mutated-{index}.toml"));
            fs::write(&manifest_path, body).expect("write mutated fixture");

            let result = std::panic::catch_unwind(|| {
                let _ = read_manifest(&manifest_path);
            });

            assert!(result.is_ok(), "mutated fixture {index} panicked");
        }
    }

    trait UnwrapErrOrElse<T> {
        fn unwrap_err_or_else(self, fallback: impl FnOnce() -> T) -> T;
    }

    impl<T, E> UnwrapErrOrElse<E> for Result<T, E> {
        fn unwrap_err_or_else(self, fallback: impl FnOnce() -> E) -> E {
            match self {
                Ok(_) => fallback(),
                Err(error) => error,
            }
        }
    }
}
