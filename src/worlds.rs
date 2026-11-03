//! The world list: the MUDs in av's cached catalog (`kind = "mud"`), plus
//! worlds the player adds by hand in `<config>/lera/worlds.toml`.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct World {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    /// connect with TLS (on `tls_port` for a catalog world that has one)
    #[serde(default)]
    pub tls: bool,
    #[serde(skip)]
    pub tls_port: Option<u16>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip)]
    pub since: Option<u32>,
    #[serde(skip)]
    pub from_catalog: bool,
}

impl World {
    /// The port and TLS setting to connect with: TLS whenever the world offers it.
    pub fn endpoint(&self) -> (u16, bool) {
        match self.tls_port {
            Some(p) => (p, true),
            None => (self.port, self.tls),
        }
    }
}

/// A world id from a name: lowercase letters and digits, dashes between words.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    s.trim_end_matches('-').to_string()
}

/// MUDs from av's cached catalog.json; empty when av has never fetched one.
pub fn from_catalog(path: &Path) -> Vec<World> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        log::warn!("catalog {} is not valid JSON", path.display());
        return vec![];
    };
    let Some(titles) = v.get("titles").and_then(|t| t.as_object()) else { return vec![] };
    let mut out: Vec<World> = titles
        .values()
        .filter(|t| t.get("kind").and_then(|k| k.as_str()) == Some("mud"))
        .filter_map(|t| {
            let c = t.get("connect")?;
            let port = |k: &str| c.get(k).and_then(|p| p.as_u64()).and_then(|p| u16::try_from(p).ok());
            Some(World {
                id: t.get("id")?.as_str()?.to_string(),
                name: t.get("title").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                host: c.get("host")?.as_str()?.to_string(),
                port: port("port")?,
                tls: false,
                // proposed in Lera's CLAUDE.md, not in SCHEMA.md yet
                tls_port: port("tls_port"),
                description: t.get("description").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                since: t.get("since").and_then(|s| s.as_u64()).map(|s| s as u32),
                from_catalog: true,
            })
        })
        .collect();
    out.sort_by_key(|w| w.name.to_lowercase());
    out
}

#[derive(Default, Serialize, Deserialize)]
struct WorldsFile {
    #[serde(default, rename = "world")]
    worlds: Vec<World>,
}

/// Worlds the player added by hand.
pub fn load_own(path: &Path) -> Vec<World> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    match toml::from_str::<WorldsFile>(&text) {
        Ok(f) => f
            .worlds
            .into_iter()
            .map(|mut w| {
                if w.id.is_empty() {
                    w.id = slug(&w.name);
                }
                w
            })
            .collect(),
        Err(e) => {
            log::warn!("{} unreadable: {e}", path.display());
            vec![]
        }
    }
}

pub fn save_own(path: &Path, worlds: &[World]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = WorldsFile { worlds: worlds.iter().filter(|w| !w.from_catalog).cloned().collect() };
    let text = format!("# Worlds added in Lera. Edit freely; Lera rereads this at start.\n\n{}", toml::to_string_pretty(&file)?);
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Discworld MUD"), "discworld-mud");
        assert_eq!(slug("  My  World! "), "my-world");
    }

    #[test]
    fn endpoint_prefers_tls() {
        let w = World { port: 23, tls_port: Some(992), ..Default::default() };
        assert_eq!(w.endpoint(), (992, true));
    }
}
