use crate::aur::client::AurClient;
use crate::config::Config;
use crate::error::{KhazaurError, Result};
use flate2::read::GzDecoder;
use git2::Repository;
use std::path::PathBuf;
use tar::Archive;
use tracing::warn;

/// Download AUR package PKGBUILD
pub async fn download_package(
    client: &AurClient,
    package_name: &str,
    config: &Config,
) -> Result<PathBuf> {
    let pkg_dir = config.clone_dir.join(package_name);
    
    // Try git clone if enabled
    if config.use_git_clone {
        match try_git_download(package_name, &pkg_dir).await {
            Ok(dir) => return Ok(dir),
            Err(e) => {
                // Check if it's a permission error - don't fall back to tarball
                if e.to_string().contains("Permission denied") || e.to_string().contains("Cannot remove existing directory") {
                    return Err(e);
                }
                warn!("Git download failed, using tarball: {}", e);
            }
        }
    }
    
    // Fall back to tarball
    download_tarball(client, package_name, config).await
}

async fn try_git_download(package_name: &str, pkg_dir: &PathBuf) -> Result<PathBuf> {
    let url = format!("https://aur.archlinux.org/{}.git", package_name);
    
    if pkg_dir.exists() {
        // Check if it's a git repository
        let git_dir = pkg_dir.join(".git");
        if git_dir.exists() {
            // It's a git repo, try to update it
            match Repository::open(pkg_dir) {
                Ok(repo) => {
                    // Fetch and reset to latest
                    match repo.find_remote("origin") {
                        Ok(mut remote) => {
                            if let Err(e) = remote.fetch(&["refs/heads/*:refs/heads/*"], None, None) {
                                warn!("Failed to fetch updates: {}, will use existing version", e);
                            } else {
                                // Reset to origin/master or origin/main
                                if let Ok(reference) = repo.find_reference("refs/heads/master") {
                                    if let Ok(commit) = reference.peel_to_commit() {
                                        let _ = repo.reset(commit.as_object(), git2::ResetType::Hard, None);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to find remote: {}, will use existing version", e);
                        }
                    }
                    return Ok(pkg_dir.clone());
                }
                Err(e) => {
                    warn!("Failed to open git repo: {}, will try to re-clone", e);
                }
            }
        }
        
        // Not a git repo or failed to update, check if there are built packages
        let has_built_packages = std::fs::read_dir(pkg_dir)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .find(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .ends_with(".pkg.tar.zst") ||
                        e.file_name()
                            .to_string_lossy()
                            .ends_with(".pkg.tar.xz")
                    })
            })
            .is_some();
        
        if has_built_packages {
            warn!("Package directory contains built packages, keeping existing directory");
            return Ok(pkg_dir.clone());
        }
        
        // Try to remove existing directory to re-clone
        if let Err(e) = std::fs::remove_dir_all(pkg_dir) {
            return Err(KhazaurError::DownloadFailed(
                format!(
                    "Cannot remove existing directory: {}\n\
                     This may be due to permission issues (files owned by root).\n\
                     Try: sudo rm -rf {:?}",
                    e, pkg_dir
                )
            ));
        }
    }
    
    // Clone repository
    Repository::clone(&url, pkg_dir)
        .map_err(|e| KhazaurError::DownloadFailed(format!("Git clone failed: {}", e)))?;
    
    Ok(pkg_dir.clone())
}

async fn download_tarball(
    client: &AurClient,
    package_name: &str,
    config: &Config,
) -> Result<PathBuf> {
    let bytes = client.download_snapshot(package_name).await?;
    let pkg_dir = config.clone_dir.join(package_name);
    
    if pkg_dir.exists() {
        // Check if there are built packages (.pkg.tar.* files)
        let has_built_packages = std::fs::read_dir(&pkg_dir)
            .ok()
            .and_then(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .find(|e| {
                        e.file_name()
                            .to_string_lossy()
                            .ends_with(".pkg.tar.zst") ||
                        e.file_name()
                            .to_string_lossy()
                            .ends_with(".pkg.tar.xz")
                    })
            })
            .is_some();
        
        if has_built_packages {
            warn!("Package directory contains built packages, keeping existing directory");
            return Ok(pkg_dir);
        }
        
        // Try to remove existing directory
        if let Err(e) = std::fs::remove_dir_all(&pkg_dir) {
            return Err(KhazaurError::DownloadFailed(
                format!(
                    "Cannot remove existing directory: {}\n\
                     This may be due to permission issues (files owned by root).\n\
                     Try: sudo rm -rf {:?}",
                    e, pkg_dir
                )
            ));
        }
    }
    
    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);
    archive.unpack(&config.clone_dir)?;
    
    if !pkg_dir.exists() {
        return Err(KhazaurError::DownloadFailed(
            format!("Package directory not found: {:?}", pkg_dir)
        ));
    }
    
    Ok(pkg_dir)
}
