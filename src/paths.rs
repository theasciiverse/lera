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

    /// av's cached catalog. av keeps one file per catalog address,
    /// `catalog-<fnv1a of the URL>.json`; av starts Lera with its own
    /// environment, so AV_CATALOG_URL names the same address here. Run on
    /// its own with another address, Lera takes the newest cached catalog.
    pub fn catalog(&self) -> PathBuf {
        let url = std::env::var("AV_CATALOG_URL").unwrap_or_else(|_| DEFAULT_CATALOG_URL.into());
        let exact = self.cache.join(format!("catalog-{:016x}.json", fnv1a(&url)));
        if exact.is_file() {
            return exact;
        }
        let newest = std::fs::read_dir(&self.cache).ok().and_then(|dir| {
            dir.flatten()
                .filter(|e| {
                    let n = e.file_name();
                    let n = n.to_string_lossy();
                    n.starts_with("catalog-") && n.ends_with(".json")
                })
                .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
                .max()
                .map(|(_, p)| p)
        });
        newest.unwrap_or(exact)
    }

    pub fn worlds_file(&self) -> PathBuf {
        self.config.join("lera").join("worlds.toml")
    }

    pub fn world_dir(&self, id: &str) -> PathBuf {
        self.state.join("lera").join(id)
    }
}

/// Where av fetches the catalog from unless AV_CATALOG_URL says otherwise
/// (av/src/catalog.rs).
const DEFAULT_CATALOG_URL: &str = "https://asciiverse.sh/catalog.json";

/// FNV-1a, as av names its cache files (av/src/catalog.rs).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cache_name_matches_av() {
        // av wrote catalog-a0a64cbba3d2002d.json for its dev catalog
        assert_eq!(format!("{:016x}", super::fnv1a("http://127.0.0.1:8765/data/catalog.json")), "a0a64cbba3d2002d");
    }
}
