use crate::error::{KhazaurError, Result};
use std::process::Command;
use tracing::info;

/// Sync package databases
pub fn sync_databases() -> Result<()> {
    info!("Syncing package databases...");
    
    let status = Command::new("sudo")
        .args(["pacman", "-Sy"])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::PacmanFailed("Database sync failed".to_string()));
    }
    
    Ok(())
}

/// Install packages from official repositories
pub fn install_packages(package_names: &[String], extra_args: &[String]) -> Result<()> {
    if package_names.is_empty() {
        return Ok(());
    }
    
    info!("Installing packages: {:?}", package_names);
    
    let mut args = vec!["pacman".to_string(), "-S".to_string()];
    args.extend_from_slice(package_names);
    args.extend_from_slice(extra_args);
    
    let status = Command::new("sudo")
        .args(&args)
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::PacmanFailed("Package installation failed".to_string()));
    }
    
    Ok(())
}

/// Upgrade all packages
pub fn upgrade_system(extra_args: &[String]) -> Result<()> {
    info!("Upgrading system...");
    
    let mut args = vec!["pacman", "-Syu"];
    let extra_str_args: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
    args.extend(&extra_str_args);
    
    let status = Command::new("sudo")
        .args(&args)
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::PacmanFailed("System upgrade failed".to_string()));
    }
    
    Ok(())
}

/// Remove packages
pub fn remove_packages(package_names: &[String], extra_args: &[String]) -> Result<()> {
    if package_names.is_empty() {
        return Ok(());
    }
    
    info!("Removing packages: {:?}", package_names);
    
    let mut args = vec!["pacman".to_string(), "-R".to_string()];
    args.extend_from_slice(package_names);
    args.extend_from_slice(extra_args);
    
    // Check if we're forcing removal (has -dd flag)
    let is_force = extra_args.iter().any(|arg| arg.contains("dd"));
    
    if is_force {
        // For forced removal, use status() to allow user interaction
        let status = Command::new("sudo")
            .args(&args)
            .status()?;
        
        if !status.success() {
            return Err(KhazaurError::PacmanFailed("Package removal failed".to_string()));
        }
    } else {
        // For normal removal, capture output to detect dependency conflicts
        let output = Command::new("sudo")
            .args(&args)
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Check if it's a dependency conflict
            if stderr.contains("could not satisfy dependencies") || stderr.contains("breaks dependency") {
                return Err(KhazaurError::PacmanFailed(format!("dependency_conflict:{}", stderr)));
            }
            
            return Err(KhazaurError::PacmanFailed(format!("Package removal failed: {}", stderr)));
        }
    }
    
    Ok(())
}

/// Install a local package file
pub fn install_local_package(file_path: &str, extra_args: &[String]) -> Result<()> {
    info!("Installing local package: {}", file_path);
    
    let mut args = vec!["pacman", "-U", file_path];
    let extra_str_args: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
    args.extend(&extra_str_args);
    
    let status = Command::new("sudo")
        .args(&args)
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::PacmanFailed("Local package installation failed".to_string()));
    }
    
    Ok(())
}


