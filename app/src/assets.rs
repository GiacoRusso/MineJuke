//! Locating the Minecraft Java assets without copying or modifying them.
//!
//! Minecraft keeps downloaded assets in a content-addressed store:
//!   assets/indexes/<version>.json
//!   assets/objects/<first two hash chars>/<hash>
//! The index tells us which object is `minecraft/sounds/records/cat.ogg`.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::disc::{Disc, DISCS};

#[derive(Debug, Deserialize)]
struct AssetIndex {
    #[serde(default)]
    objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
struct AssetObject {
    hash: String,
}

#[derive(Debug, Clone)]
pub struct AssetLibrary {
    pub index_name: String,
    /// Number of index JSON files found; shown in the setup screen.
    pub index_count: usize,
    /// disc id -> absolute path to the .ogg object.
    pub records: HashMap<String, PathBuf>,
}

impl AssetLibrary {
    /// Accepts either the `.minecraft` folder or its `assets` child.
    pub fn load(selected: &Path) -> Result<Self> {
        let assets_dir = if selected.file_name().and_then(|s| s.to_str()) == Some("assets") {
            selected.to_path_buf()
        } else {
            selected.join("assets")
        };
        let indexes_dir = assets_dir.join("indexes");
        if !indexes_dir.is_dir() {
            return Err(anyhow!(
                "No assets/indexes folder was found in {}",
                selected.display()
            ));
        }

        let (index_path, index_name, index_count) = newest_index(&indexes_dir)
            .with_context(|| format!("No asset index JSON found in {}", indexes_dir.display()))?;

        let bytes = std::fs::read(&index_path)
            .with_context(|| format!("Could not read {}", index_path.display()))?;
        let index: AssetIndex = serde_json::from_slice(&bytes)
            .with_context(|| format!("Could not parse {}", index_path.display()))?;

        let mut records = HashMap::new();
        for disc in DISCS.iter() {
            let key = format!("minecraft/sounds/records/{}.ogg", disc.asset);
            if let Some(object) = index.objects.get(&key) {
                if object.hash.len() >= 2 {
                    let prefix = &object.hash[0..2];
                    let path = assets_dir.join("objects").join(prefix).join(&object.hash);
                    if path.is_file() {
                        records.insert(disc.id.to_string(), path);
                    }
                }
            }
        }

        Ok(Self {
            index_name,
            index_count,
            records,
        })
    }

    pub fn found_count(&self) -> usize {
        self.records.len()
    }

    pub fn total_count(&self) -> usize {
        DISCS.len()
    }

    pub fn has_disc(&self, disc: &Disc) -> bool {
        self.records.contains_key(disc.id)
    }

    pub fn record_path(&self, disc: &Disc) -> Option<&Path> {
        self.records.get(disc.id).map(PathBuf::as_path)
    }
}

fn newest_index(indexes_dir: &Path) -> Result<(PathBuf, String, usize)> {
    let mut candidates: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(indexes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        candidates.push((modified, path));
    }
    if candidates.is_empty() {
        return Err(anyhow!("the folder contains no .json asset indexes"));
    }
    let count = candidates.len();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    // Prefer the newest modified index, but fall back to a numeric version
    // comparison if several indexes were copied at the same time.
    let newest = candidates
        .iter()
        .max_by(|a, b| {
            let av = version_number(&a.1);
            let bv = version_number(&b.1);
            av.cmp(&bv).then_with(|| a.0.cmp(&b.0))
        })
        .map(|(_, p)| p.clone())
        .unwrap_or_else(|| candidates[0].1.clone());
    let name = newest
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("assets")
        .to_string();
    Ok((newest, name, count))
}

fn version_number(path: &Path) -> u64 {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

/// `~/Library/Application Support/minecraft` on macOS, with sensible fallbacks.
pub fn default_minecraft_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library/Application Support/minecraft");
        }
    }
    if let Some(data) = dirs::data_dir() {
        let p = data.join("minecraft");
        if p.exists() {
            return p;
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".minecraft")
}

pub fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = path.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_is_non_empty() {
        assert!(!default_minecraft_dir().as_os_str().is_empty());
    }

    #[test]
    fn reads_real_index_when_available() {
        let dir = default_minecraft_dir();
        if !dir.join("assets/indexes").is_dir() {
            return;
        }
        let library = AssetLibrary::load(&dir).expect("default Minecraft assets should load");
        assert!(
            library.found_count() > 0,
            "expected at least one record in {}",
            dir.display()
        );
        println!(
            "loaded index {} with {}/{} records",
            library.index_name,
            library.found_count(),
            library.total_count()
        );
    }
}
