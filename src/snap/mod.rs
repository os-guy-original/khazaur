use crate::error::{KhazaurError, Result};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapPackage {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub description: String,
}

/// Check if snap is installed
pub fn is_available() -> bool {
    Command::new("which")
        .arg("snap")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Search for snap packages
pub fn search_snap(query: &str) -> Result<Vec<SnapPackage>> {
    if !is_available() {
        return Ok(Vec::new());
    }

    let output = Command::new("snap")
        .args(["find", query])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = parse_snap_search(&stdout);

    Ok(packages)
}

/// Parse snap search output
fn parse_snap_search(output: &str) -> Vec<SnapPackage> {
    let mut packages = Vec::new();

    for line in output.lines().skip(1) { // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let name = parts[0].to_string();
            let version = parts[1].to_string();
            let publisher = parts[2].to_string();
            let description = parts[3..].join(" ");

            packages.push(SnapPackage {
                name,
                version,
                publisher,
                description,
            });
        }
    }

    packages
}

/// Install a snap package
pub async fn install_snap(package_name: &str) -> Result<()> {
    use colored::Colorize;
    use tokio::process::Command;
    use tokio::signal;
    
    if !is_available() {
        return Err(KhazaurError::Config(
            "Snap is not installed on this system".to_string()
        ));
    }
    
    // Check if already installed
    if is_snap_installed(package_name)? {
        println!("{} {} {}", 
            "::".bright_blue().bold(),
            package_name.bold(),
            "is already installed".dimmed()
        );
        return Ok(());
    }
    
    println!("{} {}", "::".bright_blue().bold(), format!("Installing snap: {}", package_name).bold());
    
    let mut child = Command::new("snap")
        .args(["install", package_name])
        .spawn()
        .map_err(|e| KhazaurError::Config(format!("Failed to start snap install: {}", e)))?;
        
    tokio::select! {
        status = child.wait() => {
            match status {
                Ok(s) if s.success() => {
                    println!("{}", format!("✓ {} installed successfully", package_name).green());
                    Ok(())
                }
                Ok(_) => {
                    Err(KhazaurError::Config(format!("Snap installation failed for: {}", package_name)))
                }
                Err(e) => {
                    Err(KhazaurError::Config(format!("Failed to wait for snap process: {}", e)))
                }
            }
        }
        _ = signal::ctrl_c() => {
            println!("\n{}", ":: Installation cancelled by user".yellow());
            let _ = child.kill().await;
            Err(KhazaurError::Config("Installation cancelled".to_string()))
        }
    }
}

/// Check if a snap package is installed
pub fn is_snap_installed(package_name: &str) -> Result<bool> {
    if !is_available() {
        return Ok(false);
    }
    
    let output = Command::new("snap")
        .args(["list", package_name])
        .output()?;
    
    Ok(output.status.success())
}

/// Get list of installed snap packages matching a query
pub fn get_installed_snaps(query: &str) -> Result<Vec<String>> {
    if !is_available() {
        return Ok(Vec::new());
    }
    
    let output = Command::new("snap")
        .args(["list"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let query_lower = query.to_lowercase();
    
    let matches: Vec<String> = stdout
        .lines()
        .skip(1) // Skip header
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let name = parts[0];
                if name.to_lowercase().contains(&query_lower) {
                    return Some(name.to_string());
                }
            }
            None
        })
        .collect();
    
    Ok(matches)
}

/// Uninstall a snap package
pub fn uninstall_snap(package_name: &str) -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config(
            "Snap is not installed on this system".to_string()
        ));
    }
    
    let status = Command::new("snap")
        .args(["remove", package_name])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::Config(
            format!("Failed to uninstall snap: {}", package_name)
        ));
    }
    
    Ok(())
}

/// Parse snap info output to extract installed and available versions
/// Returns (installed_version, available_version) or None if parsing fails
fn parse_snap_versions(snap_name: &str) -> Option<(String, String)> {
    let output = Command::new("snap")
        .args(&["info", snap_name])
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let info = String::from_utf8_lossy(&output.stdout);
    let mut installed_version = None;
    let mut tracking_channel = None;
    let mut available_version = None;
    
    // First pass: find installed version and tracking channel
    for line in info.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("installed:") {
            // Format: "installed:          145.0.1-1               (7355) 262MB -"
            // Extract version (first token after "installed:")
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                installed_version = Some(parts[1].to_string());
            }
        } else if trimmed.starts_with("tracking:") {
            // Format: "tracking:     latest/stable"
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                tracking_channel = Some(parts[1].to_string());
            }
        }
    }
    
    // Second pass: find the version in the tracking channel
    if let Some(channel) = &tracking_channel {
        let channel_prefix = format!("{}:", channel);
        
        for line in info.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with(&channel_prefix) {
                // Format: "latest/stable:    145.0.2-1    2025-11-26 (7423) 262MB -"
                // Split by ':' and get the version (first token after colon)
                if let Some(version_part) = trimmed.split(':').nth(1) {
                    let parts: Vec<&str> = version_part.split_whitespace().collect();
                    if !parts.is_empty() {
                        available_version = Some(parts[0].to_string());
                        break;
                    }
                }
            }
        }
    }
    
    // Return both versions if found
    match (installed_version, available_version) {
        (Some(installed), Some(available)) => Some((installed, available)),
        _ => None,
    }
}

/// Get list of Snap packages with available updates
/// Returns Vec of (name, current_version, new_version)
pub fn get_updates() -> Result<Vec<(String, String, String)>> {
    if !is_available() {
        return Ok(Vec::new());
    }
    
    // Get list of packages that have updates available
    let output = Command::new("snap")
        .args(&["refresh", "--list"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut updates = Vec::new();
    
    for line in stdout.lines() {
        // Skip header and empty lines
        if line.is_empty() || line.starts_with("Name") || line.starts_with("All snaps") {
            continue;
        }
        
        // Extract snap name (first column)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        let name = parts[0].to_string();
        
        // Get versions for this snap
        if let Some((installed, available)) = parse_snap_versions(&name) {
            // Only add if versions are different
            if installed != available {
                updates.push((name, installed, available));
            }
        }
    }
    
    Ok(updates)
}

/// Update all Snap packages
pub fn update_all() -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config("Snap is not installed".to_string()));
    }
    
    let status = Command::new("snap")
        .args(&["refresh"])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::Config("Snap update failed".to_string()));
    }
    
    Ok(())
}
