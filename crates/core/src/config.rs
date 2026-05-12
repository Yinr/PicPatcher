use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;

/// PicPatcher configuration file (v1).
///
/// A single overlay image is pasted at absolute pixel coordinates (x, y) on
/// every matching image in the target directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    /// Path to the overlay PNG. Can be absolute, or relative to the config file directory.
    pub overlay: PathBuf,
    pub x: i64,
    pub y: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            overlay: PathBuf::from("overlay.png"),
            x: 0,
            y: 0,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let cfg: Config = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse config: {}", path.display()))?;
        if cfg.version != CONFIG_VERSION {
            anyhow::bail!(
                "unsupported config version {} (expected {})",
                cfg.version,
                CONFIG_VERSION
            );
        }
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create config directory: {}", parent.display())
                })?;
            }
        }
        std::fs::write(path, json)
            .with_context(|| format!("failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Resolve `overlay` against the config file's parent directory if relative.
    pub fn resolve_overlay(&self, config_path: &Path) -> PathBuf {
        if self.overlay.is_absolute() {
            self.overlay.clone()
        } else {
            config_path
                .parent()
                .map(|p| p.join(&self.overlay))
                .unwrap_or_else(|| self.overlay.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_overlay_against_config_dir() {
        let cfg = Config {
            version: CONFIG_VERSION,
            overlay: PathBuf::from("stamps/date.png"),
            x: 10,
            y: 20,
        };

        assert_eq!(
            cfg.resolve_overlay(Path::new("C:/project/config/picpatcher.json")),
            PathBuf::from("C:/project/config/stamps/date.png")
        );
    }

    #[test]
    fn keeps_absolute_overlay_path() {
        let overlay = PathBuf::from("C:/assets/date.png");
        let cfg = Config {
            version: CONFIG_VERSION,
            overlay: overlay.clone(),
            x: 10,
            y: 20,
        };

        assert_eq!(
            cfg.resolve_overlay(Path::new("C:/project/config.json")),
            overlay
        );
    }
}
