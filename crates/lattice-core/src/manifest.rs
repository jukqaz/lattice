use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<ManifestEntry>,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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
    toml::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))
}

#[cfg(test)]
mod tests {
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
}
