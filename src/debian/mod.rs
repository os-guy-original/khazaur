use crate::error::{KhazaurError, Result};
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebianPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub filename: String,
    pub md5sum: String,
    pub architecture: String,
    pub maintainer: Option<String>,
}

const DEBIAN_MIRROR: &str = "http://deb.debian.org/debian";
const RELEASE: &str = "bookworm";
const COMPONENT: &str = "main";

/// Get system architecture
fn get_system_arch() -> String {
    std::env::consts::ARCH.to_string()
}

/// Fetch and parse the Packages.gz index
async fn fetch_and_parse_index(show_progress: bool) -> Result<Vec<DebianPackage>> {
    let arch = get_system_arch();
    let arch_mapped = match arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => &arch,
    };
    
    // Cache the Packages.gz file
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| KhazaurError::Config("Could not find cache directory".to_string()))?
        .join("khazaur")
        .join("debian");
    
    std::fs::create_dir_all(&cache_dir)?;
    let cache_file = cache_dir.join(format!("Packages-{}-{}.gz", RELEASE, arch_mapped));
    
    // Check if cache exists and is less than 24 hours old
    let should_download = if cache_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    elapsed.as_secs() > 86400 // Re-download if older than 24 hours
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        true
    };
    
    if should_download {
        let index_url = format!(
            "{}/dists/{}/{}/binary-{}/Packages.gz",
            DEBIAN_MIRROR, RELEASE, COMPONENT, arch_mapped
        );
        
        let response = reqwest::get(&index_url).await
            .map_err(|e| KhazaurError::Config(format!("Failed to fetch Debian index: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(KhazaurError::Config(format!(
                "Failed to fetch Debian index: HTTP {}",
                response.status()
            )));
        }
        
        let bytes_vec = if show_progress {
            // Download with progress bar
            use indicatif::{ProgressBar, ProgressStyle};
            use futures_util::StreamExt;
            
            // Show message before starting download
            eprintln!("Updating Debian package index...");
            
            let total_size = response.content_length().unwrap_or(0);
            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
                .template("  [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"));
            
            let mut downloaded: u64 = 0;
            let mut bytes_vec = Vec::new();
            let mut stream = response.bytes_stream();
            
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| KhazaurError::Config(format!("Download error: {}", e)))?;
                bytes_vec.extend_from_slice(&chunk);
                downloaded += chunk.len() as u64;
                pb.set_position(downloaded);
            }
            
            pb.finish_and_clear();
            bytes_vec
        } else {
            // Download silently without any output
            response.bytes().await?.to_vec()
        };
        
        // Write to cache
        std::fs::write(&cache_file, &bytes_vec)?;
    }
    
    // Read from cache and decompress
    let bytes = std::fs::read(&cache_file)?;
    let decoder = GzDecoder::new(&bytes[..]);
    let reader = BufReader::new(decoder);
    
    // Parse packages
    let mut packages = Vec::new();
    let mut current_package = None::<DebianPackage>;
    
    for line in reader.lines() {
        let line = line?;
        
        if line.is_empty() {
            // End of package stanza
            if let Some(pkg) = current_package.take() {
                packages.push(pkg);
            }
            continue;
        }
        
        if let Some((key, value)) = line.split_once(": ") {
            let value = value.trim();
            
            match key {
                "Package" => {
                    current_package = Some(DebianPackage {
                        name: value.to_string(),
                        version: String::new(),
                        description: String::new(),
                        filename: String::new(),
                        md5sum: String::new(),
                        architecture: arch_mapped.to_string(),
                        maintainer: None,
                    });
                }
                "Version" => {
                    if let Some(ref mut pkg) = current_package {
                        pkg.version = value.to_string();
                    }
                }
                "Description" => {
                    if let Some(ref mut pkg) = current_package {
                        pkg.description = value.to_string();
                    }
                }
                "Filename" => {
                    if let Some(ref mut pkg) = current_package {
                        pkg.filename = value.to_string();
                    }
                }
                "MD5sum" => {
                    if let Some(ref mut pkg) = current_package {
                        pkg.md5sum = value.to_string();
                    }
                }
                "Maintainer" => {
                    if let Some(ref mut pkg) = current_package {
                        pkg.maintainer = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    
    // Add last package if exists
    if let Some(pkg) = current_package {
        packages.push(pkg);
    }
    
    Ok(packages)
}

/// Update Debian package index (with progress bar)
pub async fn update_index() -> Result<()> {
    fetch_and_parse_index(true).await?;
    Ok(())
}

/// Check if Debian index needs updating
pub fn index_needs_update() -> bool {
    let cache_dir = match dirs::cache_dir() {
        Some(dir) => dir.join("khazaur").join("debian"),
        None => return true,
    };
    
    let arch = get_system_arch();
    let arch_mapped = match arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => &arch,
    };
    
    let cache_file = cache_dir.join(format!("Packages-{}-{}.gz", RELEASE, arch_mapped));
    
    if !cache_file.exists() {
        return true;
    }
    
    // Check if cache is older than 24 hours
    if let Ok(metadata) = std::fs::metadata(&cache_file) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed.as_secs() > 86400;
            }
        }
    }
    
    false
}

/// Search for Debian packages matching a query
pub async fn search_debian(query: &str) -> Result<Vec<DebianPackage>> {
    let all_packages = fetch_and_parse_index(false).await?;
    let query_lower = query.to_lowercase();
    
    let matches: Vec<DebianPackage> = all_packages
        .into_iter()
        .filter(|pkg| pkg.name.to_lowercase().contains(&query_lower))
        .collect();
    
    Ok(matches)
}

/// Download a Debian package and verify its checksum
pub async fn download_debian(package: &DebianPackage) -> Result<PathBuf> {
    use std::fs;
    use std::io::Write;
    
    // Create cache directory
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| KhazaurError::Config("Could not find cache directory".to_string()))?
        .join("khazaur")
        .join("debian");
    
    fs::create_dir_all(&cache_dir)?;
    
    let download_url = format!("{}/{}", DEBIAN_MIRROR, package.filename);
    let filename = package.filename.split('/').last().unwrap_or(&package.filename);
    let output_path = cache_dir.join(filename);
    
    // Download
    println!("Downloading {} from Debian repository...", package.name);
    let response = reqwest::get(&download_url).await
        .map_err(|e| KhazaurError::Config(format!("Failed to download package: {}", e)))?;
    
    if !response.status().is_success() {
        return Err(KhazaurError::Config(format!(
            "Failed to download package: HTTP {}",
            response.status()
        )));
    }
    
    let bytes = response.bytes().await
        .map_err(|e| KhazaurError::Config(format!("Failed to read package data: {}", e)))?;
    
    // Write to file
    let mut file = fs::File::create(&output_path)?;
    file.write_all(&bytes)?;
    
    // Verify MD5 checksum
    let calculated_md5 = format!("{:x}", md5::compute(&bytes));
    
    if calculated_md5 != package.md5sum {
        fs::remove_file(&output_path)?;
        return Err(KhazaurError::Config(format!(
            "MD5 checksum mismatch! Expected: {}, Got: {}",
            package.md5sum, calculated_md5
        )));
    }
    
    println!("✓ Package downloaded and verified: {}", output_path.display());
    Ok(output_path)
}

/// Get the path to the Debian tracking file
fn get_tracking_file() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| KhazaurError::Config("Could not find config directory".to_string()))?
        .join("khazaur");
    
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("debian_packages.txt"))
}

/// Track a package as installed from Debian
pub fn track_debian_package(package_name: &str) -> Result<()> {
    let tracking_file = get_tracking_file()?;
    let mut packages = load_tracked_packages()?;
    packages.insert(package_name.to_string());
    
    let mut file = std::fs::File::create(tracking_file)?;
    for pkg in packages {
        writeln!(file, "{}", pkg)?;
    }
    Ok(())
}

/// Load tracked Debian packages
fn load_tracked_packages() -> Result<HashSet<String>> {
    let tracking_file = get_tracking_file()?;
    
    if !tracking_file.exists() {
        return Ok(HashSet::new());
    }
    
    let content = std::fs::read_to_string(tracking_file)?;
    Ok(content.lines().map(|s| s.to_string()).collect())
}

/// Check if a package was installed from Debian
#[allow(dead_code)]
pub fn is_debian_package(package_name: &str) -> bool {
    load_tracked_packages()
        .map(|packages| packages.contains(package_name))
        .unwrap_or(false)
}

/// Check for Debian package updates
/// Returns list of (package_name, installed_version, debian_version, debian_package)
pub async fn check_debian_updates() -> Result<Vec<(String, String, String, DebianPackage)>> {
    // Get tracked Debian packages
    let tracked_packages = load_tracked_packages()?;
    
    // Get all installed packages that might be from Debian
    let installed = crate::pacman::get_installed_packages()?;
    
    // Fetch Debian package index
    let debian_packages = fetch_and_parse_index(true).await?;
    
    let mut updates = Vec::new();
    
    for (pkg_name, installed_version) in installed {
        // Only check packages that were tracked as Debian packages
        if !tracked_packages.contains(&pkg_name) {
            continue;
        }
        
        // Find matching Debian package
        if let Some(debian_pkg) = debian_packages.iter().find(|p| p.name == pkg_name) {
            // Compare versions using vercmp
            if needs_update(&installed_version, &debian_pkg.version)? {
                updates.push((
                    pkg_name,
                    installed_version,
                    debian_pkg.version.clone(),
                    debian_pkg.clone(),
                ));
            }
        }
    }
    
    Ok(updates)
}

/// Check if a package needs an update by comparing versions
fn needs_update(installed_version: &str, available_version: &str) -> Result<bool> {
    use std::process::Command;
    
    let output = Command::new("vercmp")
        .arg(installed_version)
        .arg(available_version)
        .output()?;
    
    if !output.status.success() {
        return Ok(false);
    }
    
    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // vercmp returns:
    // -1 if installed < available (update needed)
    //  0 if installed == available (no update)
    //  1 if installed > available (downgrade, no update)
    Ok(result == "-1")
}
