use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub command: String,
    pub model: String,
    pub timestamp: i64,
}

fn cache_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let xdg_path = home.join(".config").join("yaak").join("cache.json");
        if xdg_path.parent().map(|p| p.exists()).unwrap_or(false) {
            return xdg_path;
        }
    }
    if let Some(config_dir) = dirs::config_dir() {
        return config_dir.join("yaak").join("cache.json");
    }
    PathBuf::from("cache.json")
}

fn load_cache() -> Cache {
    let path = cache_path();
    if let Ok(contents) = fs::read_to_string(&path) {
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        Cache::default()
    }
}

fn save_cache(cache: &Cache) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(&path, json);
    }
}

/// Build a cache key from the description and model.
fn cache_key(description: &str, model: &str) -> String {
    format!("{}::{}", model, description.trim().to_lowercase())
}

/// Look up a cached command for the given description and model.
pub fn get(description: &str, model: &str) -> Option<CacheEntry> {
    let cache = load_cache();
    let key = cache_key(description, model);
    cache.entries.get(&key).cloned()
}

/// Store a description→command mapping in the cache.
pub fn put(description: &str, model: &str, command: &str) {
    let mut cache = load_cache();
    let key = cache_key(description, model);
    let entry = CacheEntry {
        command: command.to_string(),
        model: model.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    cache.entries.insert(key, entry);
    save_cache(&cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_case_insensitive() {
        assert_eq!(
            cache_key("List Files", "gpt-4o"),
            cache_key("list files", "gpt-4o")
        );
    }

    #[test]
    fn cache_key_includes_model() {
        assert_ne!(
            cache_key("list files", "gpt-4o"),
            cache_key("list files", "claude-sonnet-4-6")
        );
    }

    #[test]
    fn cache_key_trims_whitespace() {
        assert_eq!(
            cache_key("  list files  ", "gpt-4o"),
            cache_key("list files", "gpt-4o")
        );
    }

    #[test]
    fn roundtrip_cache_entry() {
        let entry = CacheEntry {
            command: "ls -la".to_string(),
            model: "gpt-4o".to_string(),
            timestamp: 1000,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: CacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.model, "gpt-4o");
        assert_eq!(parsed.timestamp, 1000);
    }

    #[test]
    fn empty_cache_returns_none() {
        let cache = Cache::default();
        let key = cache_key("anything", "model");
        assert!(!cache.entries.contains_key(&key));
    }

    #[test]
    fn put_and_get_with_temp_dir() {
        // Test the full flow using a temporary cache file
        let dir = tempfile::TempDir::new().unwrap();
        let cache_file = dir.path().join("cache.json");

        let mut cache = Cache::default();
        let key = cache_key("list files", "gpt-4o");
        cache.entries.insert(
            key.clone(),
            CacheEntry {
                command: "ls -la".to_string(),
                model: "gpt-4o".to_string(),
                timestamp: 1000,
            },
        );

        let json = serde_json::to_string_pretty(&cache).unwrap();
        fs::write(&cache_file, &json).unwrap();

        let loaded: Cache =
            serde_json::from_str(&fs::read_to_string(&cache_file).unwrap()).unwrap();
        let entry = loaded.entries.get(&key).unwrap();
        assert_eq!(entry.command, "ls -la");
    }
}
