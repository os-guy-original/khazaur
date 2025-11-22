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
