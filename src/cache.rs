use crate::cli::PackageCandidate;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_DURATION_SECS: u64 = 3600; // 1 hour

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    timestamp: u64,
    candidates: Vec<PackageCandidate>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SearchCache {
    entries: HashMap<String, CacheEntry>,
}

impl SearchCache {
    fn get_cache_path() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| crate::error::KhazaurError::Config("Could not find cache directory".to_string()))?
            .join("khazaur");
        
        std::fs::create_dir_all(&cache_dir)?;
        Ok(cache_dir.join("search_cache.json"))
    }
    
    fn load() -> Self {
        let cache_path = match Self::get_cache_path() {
            Ok(path) => path,
            Err(_) => return Self::default(),
        };
        
        if !cache_path.exists() {
            return Self::default();
        }
        
        match std::fs::read_to_string(&cache_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
    
    fn save(&self) -> Result<()> {
        let cache_path = Self::get_cache_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)?;
        Ok(())
    }
    
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        let now = Self::current_timestamp();
        now - entry.timestamp > CACHE_DURATION_SECS
    }
    
    pub fn get(&self, package_name: &str) -> Option<&Vec<PackageCandidate>> {
        if let Some(entry) = self.entries.get(package_name) {
            if !self.is_expired(entry) {
                return Some(&entry.candidates);
            }
        }
        None
    }
    
    pub fn set(&mut self, package_name: String, candidates: Vec<PackageCandidate>) {
        let entry = CacheEntry {
            timestamp: Self::current_timestamp(),
            candidates,
        };
        self.entries.insert(package_name, entry);
    }
    
    pub fn clear_expired(&mut self) {
        let now = Self::current_timestamp();
        self.entries.retain(|_, entry| {
            now - entry.timestamp <= CACHE_DURATION_SECS
        });
    }
}

/// Get cached search results for a package
pub fn get_cached_search(package_name: &str) -> Option<Vec<PackageCandidate>> {
    let cache = SearchCache::load();
    cache.get(package_name).cloned()
}

/// Cache search results for a package
pub fn cache_search_results(package_name: String, candidates: Vec<PackageCandidate>) -> Result<()> {
    let mut cache = SearchCache::load();
    cache.clear_expired();
    cache.set(package_name, candidates);
    cache.save()?;
    Ok(())
}

/// Clear all cached search results
#[allow(dead_code)]
pub fn clear_search_cache() -> Result<()> {
    let cache_path = SearchCache::get_cache_path()?;
    if cache_path.exists() {
        std::fs::remove_file(cache_path)?;
    }
    Ok(())
}
