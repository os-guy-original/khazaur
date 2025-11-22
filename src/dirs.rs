use crate::error::Result;
use std::path::PathBuf;

/// Get the khazaur cache directory
pub fn cache_dir() -> Result<PathBuf> {
    Ok(dirs::cache_dir()
        .ok_or_else(|| crate::error::KhazaurError::Config("Could not determine cache directory".to_string()))?
        .join("khazaur"))
}

/// Get the clone directory for PKGBUILDs
#[allow(dead_code)]
pub fn clone_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join("clone"))
}

/// Get the package cache directory
#[allow(dead_code)]
pub fn pkg_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join("pkg"))
}
