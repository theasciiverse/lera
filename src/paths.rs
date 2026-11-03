//! av's four per-OS buckets (av/src/paths.rs), so Lera keeps its files
//! beside av's instead of in a folder of its own:
//!
//! | | Linux | macOS | Windows |
//! |---|---|---|---|
//! | state | `~/.local/share/asciiverse` | `~/Library/Application Support/Asciiverse` | `%LOCALAPPDATA%\Asciiverse` |
//! | cache | `~/.cache/asciiverse` | `~/Library/Caches/Asciiverse` | `%LOCALAPPDATA%\Asciiverse\cache` |
//! | config | `~/.config/asciiverse` | `~/Library/Application Support/Asciiverse` | `%APPDATA%\Asciiverse` |

use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::BaseDirs;

pub struct Paths {
    /// per-world data: `<state>/lera/<world-id>/`
    pub state: PathBuf,
    /// av's `catalog.json`, and `lera.log`
    pub cache: PathBuf,
    /// `<config>/lera/worlds.toml`
    pub config: PathBuf,
}

const NAME: &str = if cfg!(any(target_os = "macos", windows)) { "Asciiverse" } else { "asciiverse" };

impl Paths {
    pub fn new() -> Result<Self> {
        let b = BaseDirs::new().context("no home directory")?;
        let data = b.data_local_dir().join(NAME);
        let cache = if cfg!(windows) { data.join("cache") } else { b.cache_dir().join(NAME) };
        let config = b.config_dir().join(NAME);
        std::fs::create_dir_all(&cache)?;
        Ok(Paths { state: data, cache, config })
    }

    pub fn catalog(&self) -> PathBuf {
        self.cache.join("catalog.json")
    }

    pub fn worlds_file(&self) -> PathBuf {
        self.config.join("lera").join("worlds.toml")
    }

    pub fn world_dir(&self, id: &str) -> PathBuf {
        self.state.join("lera").join(id)
    }
}
