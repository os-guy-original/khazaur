use crate::error::{KhazaurError, Result};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatpakPackage {
    pub name: String,
    pub app_id: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub description: String,
}

/// Check if flatpak is installed
pub fn is_available() -> bool {
    Command::new("which")
        .arg("flatpak")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Search for flatpak packages
pub fn search_flatpak(query: &str, no_timeout: bool) -> Result<Vec<FlatpakPackage>> {
    if !is_available() {
        return Ok(Vec::new());
    }

    // Set a timeout to prevent hanging (unless disabled)
    use std::process::Stdio;
    
    let output = if no_timeout {
        // No timeout - run flatpak search directly
        Command::new("flatpak")
            .args(["search", "--columns=name,description,application,version,branch", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?
    } else {
        // Use timeout command to prevent hanging
        let timeout_result = Command::new("timeout")
            .args(["5", "flatpak", "search", "--columns=name,description,application,version,branch", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match timeout_result {
            Ok(o) => o,
            Err(_) => {
                // Try without timeout command (might not be available)
                let o = Command::new("flatpak")
                    .args(["search", "--columns=name,description,application,version,branch", query])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?;
                o
            }
        }
    };

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = parse_flatpak_search(&stdout);

    Ok(packages)
}

/// Parse flatpak search output
fn parse_flatpak_search(output: &str) -> Vec<FlatpakPackage> {
    let mut packages = Vec::new();

    for line in output.lines() {
        // Skip empty lines and headers
        if line.trim().is_empty() || line.starts_with("Name") {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].trim();
            let description = parts.get(1).map_or("", |v| v.trim());
            let app_id = parts.get(2).map_or(name, |v| v.trim());
            let version = parts.get(3).map_or("", |v| v.trim());
            let branch = parts.get(4).map_or("stable", |v| v.trim());
            // Origin not in output, default to flathub
            let origin = "flathub";

            packages.push(FlatpakPackage {
                name: name.to_string(),
                description: description.to_string(),
                app_id: app_id.to_string(),
                version: version.to_string(),
                branch: branch.to_string(),
                origin: origin.to_string(),
            });
        }
    }

    packages
}

/// Install a flatpak application
pub async fn install_flatpak(app_id: &str) -> Result<()> {
    use colored::Colorize;
    use tokio::process::Command;
    use tokio::signal;
    
    if !is_available() {
        return Err(KhazaurError::Config(
            "Flatpak is not installed on this system".to_string()
        ));
    }
    
    // Check if already installed
    if is_flatpak_installed(app_id)? {
        println!("{} {} {}", 
            "::".bright_blue().bold(),
            app_id.bold(),
            "is already installed".dimmed()
        );
        return Ok(());
    }
    
    println!("{} {}", "::".bright_blue().bold(), format!("Installing flatpak: {}", app_id).bold());
    
    let mut child = Command::new("flatpak")
        .args(["install", "-y", "flathub", app_id])
        .spawn()
        .map_err(|e| KhazaurError::Config(format!("Failed to start flatpak install: {}", e)))?;
        
    tokio::select! {
        status = child.wait() => {
            match status {
                Ok(s) if s.success() => {
                    println!("{}", format!("✓ {} installed successfully", app_id).green());
                    Ok(())
                }
                Ok(_) => {
                    Err(KhazaurError::Config(format!("Failed to install flatpak: {}", app_id)))
                }
                Err(e) => {
                    Err(KhazaurError::Config(format!("Failed to wait for flatpak process: {}", e)))
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

/// Check if a flatpak application is installed
pub fn is_flatpak_installed(app_id: &str) -> Result<bool> {
    if !is_available() {
        return Ok(false);
    }
    
    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()?;
    
    if !output.status.success() {
        return Ok(false);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line.trim() == app_id))
}

/// Get list of installed flatpak applications matching a query
pub fn get_installed_flatpaks(query: &str) -> Result<Vec<String>> {
    if !is_available() {
        return Ok(Vec::new());
    }
    
    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let query_lower = query.to_lowercase();
    
    let matches: Vec<String> = stdout
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|s| s.trim().to_string())
        .collect();
    
    Ok(matches)
}

/// Uninstall a flatpak application
pub fn uninstall_flatpak(app_id: &str) -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config(
            "Flatpak is not installed on this system".to_string()
        ));
    }
    
    let status = Command::new("flatpak")
        .args(["uninstall", "-y", app_id])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::Config(
            format!("Failed to uninstall flatpak: {}", app_id)
        ));
    }
    
    Ok(())
}
