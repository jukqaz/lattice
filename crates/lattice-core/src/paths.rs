use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatticePaths {
    pub config_home: PathBuf,
    pub data_home: PathBuf,
    pub state_home: PathBuf,
    pub cache_home: PathBuf,
    pub config_file: PathBuf,
    pub services_dir: PathBuf,
    pub repo_cache_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl LatticePaths {
    pub fn discover() -> Result<Self> {
        let home = env::var_os("HOME").context("HOME is not set")?;
        Ok(Self::from_env(Path::new(&home), |key| env::var_os(key)))
    }

    pub fn from_env<F>(home: &Path, get_env: F) -> Self
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let config_home = xdg_dir(home, &get_env, "XDG_CONFIG_HOME", ".config");
        let data_home = xdg_dir(home, &get_env, "XDG_DATA_HOME", ".local/share");
        let state_home = xdg_dir(home, &get_env, "XDG_STATE_HOME", ".local/state");
        let cache_home = xdg_dir(home, &get_env, "XDG_CACHE_HOME", ".cache");

        Self {
            config_file: config_home.join("lattice").join("lattice.toml"),
            services_dir: config_home.join("lattice").join("services"),
            repo_cache_dir: data_home.join("lattice").join("repos"),
            state_dir: state_home.join("lattice"),
            cache_dir: cache_home.join("lattice"),
            config_home,
            data_home,
            state_home,
            cache_home,
        }
    }
}

fn xdg_dir<F>(home: &Path, get_env: &F, key: &str, fallback: &str) -> PathBuf
where
    F: Fn(&str) -> Option<OsString>,
{
    match get_env(key) {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => home.join(fallback),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use super::LatticePaths;

    #[test]
    fn falls_back_to_xdg_default_locations_under_home() {
        let home = Path::new("/tmp/lattice-home");
        let paths = LatticePaths::from_env(home, |_| None);

        assert_eq!(
            paths.config_file,
            Path::new("/tmp/lattice-home/.config/lattice/lattice.toml")
        );
        assert_eq!(
            paths.services_dir,
            Path::new("/tmp/lattice-home/.config/lattice/services")
        );
        assert_eq!(
            paths.repo_cache_dir,
            Path::new("/tmp/lattice-home/.local/share/lattice/repos")
        );
        assert_eq!(
            paths.state_dir,
            Path::new("/tmp/lattice-home/.local/state/lattice")
        );
        assert_eq!(
            paths.cache_dir,
            Path::new("/tmp/lattice-home/.cache/lattice")
        );
    }

    #[test]
    fn honors_xdg_environment_overrides() {
        let home = Path::new("/tmp/lattice-home");
        let paths = LatticePaths::from_env(home, |key| match key {
            "XDG_CONFIG_HOME" => Some(OsString::from("/cfg")),
            "XDG_DATA_HOME" => Some(OsString::from("/data")),
            "XDG_STATE_HOME" => Some(OsString::from("/state")),
            "XDG_CACHE_HOME" => Some(OsString::from("/cache")),
            _ => None,
        });

        assert_eq!(paths.config_file, Path::new("/cfg/lattice/lattice.toml"));
        assert_eq!(paths.services_dir, Path::new("/cfg/lattice/services"));
        assert_eq!(paths.repo_cache_dir, Path::new("/data/lattice/repos"));
        assert_eq!(paths.state_dir, Path::new("/state/lattice"));
        assert_eq!(paths.cache_dir, Path::new("/cache/lattice"));
    }
}
