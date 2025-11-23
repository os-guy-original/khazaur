use crate::aur::AurClient;
use crate::cli::{PackageCandidate, PackageSource};
use crate::config::Config;
use crate::error::Result;
use crate::flatpak;
use crate::pacman;
use crate::snap;
use indicatif::ProgressBar;
use tracing::debug;

/// Find all sources where a package is available
pub async fn find_package_sources(
    package_name: &str,
   client: &AurClient,
    _config: &Config,
    only_aur: bool,
    only_repos: bool,
    only_flatpak: bool,
    only_snap: bool,
    only_debian: bool,
    no_timeout: bool,
    spinner: Option<&ProgressBar>,
) -> Result<Vec<PackageCandidate>> {
    let mut candidates = Vec::new();
    
    // If no specific source is requested, search all
    let search_all = !only_aur && !only_repos && !only_flatpak && !only_snap && !only_debian;
    
    // Check if it's in official repos
    if search_all || only_repos {
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching repositories for '{}'... - {} found", package_name, candidates.len()));
        }
        debug!("Checking official repositories for '{}'", package_name);
        
        // Use search_repos to get repository info
        match pacman::search_repos(package_name) {
            Ok(packages) => {
                let mut found = false;
                for pkg in packages {
                    if pkg.name == package_name {
                        debug!("Found '{}' in official repositories ({})", package_name, pkg.repository);
                        candidates.push(PackageCandidate {
                            name: package_name.to_string(),
                            source: PackageSource::Repo(pkg),
                        });
                        found = true;
                        break;
                    }
                }
                
                if !found {
                    // Fallback to get_package_details if search fails but package exists
                    // This handles cases where search might behave differently or package is installed but not in sync DB
                    if let Ok(Some(pkg)) = pacman::get_package_details(package_name) {
                         debug!("Found '{}' in official repositories (details)", package_name);
                         candidates.push(PackageCandidate {
                            name: package_name.to_string(),
                            source: PackageSource::Repo(pkg),
                        });
                    } else {
                        debug!("Not found in official repositories");
                    }
                }
            }
            Err(e) => {
                debug!("Repo search error: {}", e);
                // Fallback check
                if let Ok(Some(pkg)) = pacman::get_package_details(package_name) {
                     candidates.push(PackageCandidate {
                        name: package_name.to_string(),
                        source: PackageSource::Repo(pkg),
                    });
                }
            }
        }
        
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching repositories for '{}'... - {} found", package_name, candidates.len()));
        }
    }
    
    // Check AUR
    if search_all || only_aur {
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching AUR for '{}'... - {} found", package_name, candidates.len()));
        }
        debug!("Checking AUR for '{}'", package_name);
        
        match client.info(package_name).await {
            Ok(pkg) => {
                debug!("{} found in AUR", package_name);
                candidates.push(PackageCandidate {
                    name: package_name.to_string(),
                    source: PackageSource::Aur(pkg),
                });
            }
            Err(_) => {
                debug!("Not found in AUR");
            }
        }
        
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching AUR for '{}'... - {} found", package_name, candidates.len()));
        }
    }
    
    // Check Flatpak (only if available)
    if (search_all || only_flatpak) && flatpak::is_available() {
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching Flatpak for '{}'... - {} found", package_name, candidates.len()));
        }
        debug!("Checking Flatpak for '{}'", package_name);
        
        match flatpak::search_flatpak(package_name, no_timeout) {
            Ok(packages) => {
                for pkg in packages {
                    // Match if query appears in name (case-insensitive) or exact app_id match
                    let query_lower = package_name.to_lowercase();
                    let name_lower = pkg.name.to_lowercase();
                    let app_id_lower = pkg.app_id.to_lowercase();
                    
                    if name_lower.contains(&query_lower) || app_id_lower == query_lower {
                        debug!("Found '{}' in Flatpak: {}", package_name, pkg.app_id);
                        candidates.push(PackageCandidate {
                            name: package_name.to_string(),
                            source: PackageSource::Flatpak(pkg),
                        });
                    }
                }
            }
            Err(e) => {
                debug!("Flatpak search error: {}", e);
            }
        }
        
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching Flatpak for '{}'... - {} found", package_name, candidates.len()));
        }
    }
    
    // Check Snap (only if available)
    if (search_all || only_snap) && snap::is_available() {
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching Snap for '{}'... - {} found", package_name, candidates.len()));
        }
        debug!("Checking Snap for '{}'", package_name);
        
        match snap::search_snap(package_name) {
            Ok(packages) => {
                for pkg in packages {
                    // Match if query appears in name (case-insensitive)
                    let query_lower = package_name.to_lowercase();
                    let name_lower = pkg.name.to_lowercase();
                    
                    if name_lower.contains(&query_lower) {
                        debug!("Found '{}' in Snap: {}", package_name, pkg.name);
                        candidates.push(PackageCandidate {
                            name: package_name.to_string(),
                            source: PackageSource::Snap(pkg),
                        });
                    }
                }
            }
            Err(e) => {
                debug!("Snap search error: {}", e);
            }
        }
        
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching Snap for '{}'... - {} found", package_name, candidates.len()));
        }
    }
    
    // Check Debian (only if debtap is available)
    if (search_all || only_debian) && crate::debtap::is_available() {
        if let Some(sp) = spinner {
            let msg = if crate::debian::index_needs_update() {
                format!("Searching Debian for '{}'... (updating index) - {} found", package_name, candidates.len())
            } else {
                format!("Searching Debian for '{}'... - {} found", package_name, candidates.len())
            };
            sp.set_message(msg);
        }
        debug!("Checking Debian for '{}'", package_name);
        
        match crate::debian::search_debian(package_name).await {
            Ok(packages) => {
                for pkg in packages {
                    debug!("{} found in Debian", pkg.name);
                    candidates.push(PackageCandidate {
                        name: pkg.name.clone(),
                        source: PackageSource::Debian(pkg),
                    });
                }
            }
            Err(e) => {
                debug!("Error searching Debian: {}", e);
            }
        }
        
        if let Some(sp) = spinner {
            sp.set_message(format!("Searching Debian for '{}'... - {} found", package_name, candidates.len()));
        }
    }
    
    Ok(candidates)
}
