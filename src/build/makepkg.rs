use crate::error::{KhazaurError, Result};

use std::path::Path;
use std::process::Command;
use tracing::info;

/// Build and install a package using makepkg
pub fn build_and_install(package_dir: &Path, install: bool) -> Result<()> {
    info!("Building package in {:?}", package_dir);
    
    // Check if PKGBUILD exists
    let pkgbuild = package_dir.join("PKGBUILD");
    if !pkgbuild.exists() {
        return Err(KhazaurError::BuildFailed(
            "PKGBUILD not found".to_string(),
        ));
    }
    
    // Build arguments
    let mut args = vec!["-s"]; // Install dependencies
    if install {
        args.push("-i"); // Install after building
    }
    
    // Just run makepkg directly - let it handle all output/prompts
    let status = Command::new("makepkg")
        .args(&args)
        .current_dir(package_dir)
        .status()?;
    
    if !status.success() {
        let exit_code = status.code().unwrap_or(-1);
        
        // Exit code 8 typically means dependency resolution failed
        if exit_code == 8 {
            return Err(KhazaurError::BuildFailed(
                "\nDependency installation failed.\n\n\
                 This can happen if you:\n\
                 • Interrupted the operation (Ctrl+C)\n\
                 • Rejected removing a conflicting package\n\
                 • Have network/download issues\n\n\
                 Try: khazaur -S <deps> to install dependencies manually".to_string()
            ));
        }
        
        return Err(KhazaurError::BuildFailed(
            format!("makepkg failed with status: {}", status),
        ));
    }
    
    info!("Package built successfully");
    Ok(())
}


