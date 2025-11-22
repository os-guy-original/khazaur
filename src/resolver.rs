use crate::aur::AurPackage;
use crate::error::Result;
use crate::pacman;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use tracing::debug;

/// Dependency resolver
pub struct Resolver {
    /// Packages already resolved
    resolved: HashSet<String>,
    /// Resolution order
    order: Vec<String>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            resolved: HashSet::new(),
            order: Vec::new(),
        }
    }

    /// Resolve dependencies for AUR packages
    pub async fn resolve(
        &mut self,
        packages: &[AurPackage],
        aur_client: &crate::aur::AurClient,
    ) -> Result<Vec<String>> {
        let mut aur_map: HashMap<String, AurPackage> = HashMap::new();
        for pkg in packages {
            aur_map.insert(pkg.name.clone(), pkg.clone());
        }

        // Build dependency graph
        for pkg in packages {
            self.resolve_package(&pkg.name, &aur_map, aur_client).await?;
        }

        Ok(self.order.clone())
    }

    /// Resolve a single package and its dependencies
    fn resolve_package<'a>(
        &'a mut self,
        package_name: &'a str,
        aur_map: &'a HashMap<String, AurPackage>,
        aur_client: &'a crate::aur::AurClient,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
        // Skip if already resolved
        if self.resolved.contains(package_name) {
            return Ok(());
        }

        // Get package info
        let pkg = if let Some(p) = aur_map.get(package_name) {
            p.clone()
        } else {
            // Try to fetch from AUR
            match aur_client.info(package_name).await {
                Ok(p) => p,
                Err(_) => {
                    // Might be in official repos, skip
                    debug!("{} is in official repos or not found", package_name);
                    return Ok(());
                }
            }
        };

        // Resolve dependencies first
        for dep in &pkg.all_depends() {
            let dep_name = extract_package_name(dep);
            
            // Skip if in official repos or already installed
            if pacman::is_installed(&dep_name).unwrap_or(false) {
                debug!("{} is already installed", dep_name);
                continue;
            }
            
            // Check if in official repos
            if is_in_repos(&dep_name) {
                debug!("{} is in official repos", dep_name);
                continue;
            }
            
            // Recursively resolve AUR dependency
            self.resolve_package(&dep_name, aur_map, aur_client).await?;
        }

        // Add this package to resolution order
        self.resolved.insert(package_name.to_string());
        self.order.push(package_name.to_string());

        Ok(())
        })
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract package name from dependency string (remove version specifiers)
fn extract_package_name(dep: &str) -> String {
    dep.split(&['<', '>', '=', ' '][..])
        .next()
        .unwrap_or(dep)
        .to_string()
}

/// Check if package is in official repositories
fn is_in_repos(package_name: &str) -> bool {
    pacman::get_repo_info(package_name)
        .ok()
        .flatten()
        .is_some()
}
