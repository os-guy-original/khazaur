use crate::error::{KhazaurError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Cache directory for khazaur
    #[serde(skip)]
    pub cache_dir: PathBuf,
    
    /// Clone directory for PKGBUILDs
    #[serde(skip)]
    pub clone_dir: PathBuf,
    
    /// Package cache directory
    #[serde(skip)]
    pub pkg_dir: PathBuf,
    
    /// Whether to use colors in output
    pub use_color: bool,
    
    /// Whether to ask for confirmation before operations
    pub confirm: bool,
    
    /// Whether to review PKGBUILDs before building
    pub review_pkgbuild: bool,
    
    /// Number of concurrent downloads
    pub concurrent_downloads: usize,

    /// Default text editor for editing PKGBUILDs
    pub default_editor: Option<String>,

    /// Use git clone instead of tarball download (faster)
    pub use_git_clone: bool,

    /// Maximum concurrent AUR RPC requests
    pub max_concurrent_requests: usize,

    /// Delay between requests in milliseconds
    pub request_delay_ms: u64,
    
    /// Track which optional dependencies user has rejected
    #[serde(default)]
    pub rejected_dependencies: RejectedDependencies,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RejectedDependencies {
    #[serde(default)]
    pub flatpak: bool,
    #[serde(default)]
    pub snapd: bool,
    #[serde(default)]
    pub debtap: bool,
}

impl Config {
    /// Create a new config with default values
    pub fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| KhazaurError::Config("Could not determine cache directory".to_string()))?
            .join("khazaur");
        
        let clone_dir = cache_dir.join("clone");
        let pkg_dir = cache_dir.join("pkg");
        
        Ok(Self {
            cache_dir,
            clone_dir,
            pkg_dir,
            use_color: true,
            confirm: true,
            review_pkgbuild: false,
            concurrent_downloads: 4,
            default_editor: None,
            use_git_clone: true,
            max_concurrent_requests: 10,
            request_delay_ms: 100,
            rejected_dependencies: RejectedDependencies::default(),
        })
    }

    /// Get the config file path
    pub fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| KhazaurError::Config("Could not determine config directory".to_string()))?
            .join("khazaur");
        
        Ok(config_dir.join("config.toml"))
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        let mut config = if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            toml::from_str(&contents).map_err(|e| {
                KhazaurError::Config(format!("Failed to parse config: {}", e))
            })?
        } else {
            Self::new()?
        };

        // Set runtime paths (not serialized)
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| KhazaurError::Config("Could not determine cache directory".to_string()))?
            .join("khazaur");
        
        config.cache_dir = cache_dir.clone();
        config.clone_dir = cache_dir.join("clone");
        config.pkg_dir = cache_dir.join("pkg");

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        
        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self).map_err(|e| {
            KhazaurError::Config(format!("Failed to serialize config: {}", e))
        })?;

        fs::write(&config_path, toml_string)?;
        Ok(())
    }

    /// Ensure all directories exist
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.clone_dir)?;
        std::fs::create_dir_all(&self.pkg_dir)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new().expect("Failed to create default config")
    }
}
