use crate::error::{KhazaurError, Result};
use crate::ui;
use std::path::Path;
use std::process::Command;

/// Check if debtap is installed
pub fn is_available() -> bool {
    which::which("debtap").is_ok()
}

/// Update debtap database (debtap -u)
pub fn update_database() -> Result<()> {
    if !is_available() {
        return Ok(());
    }

    println!("{}", ui::section_header("Updating Debtap Database"));
    
    let status = Command::new("sudo")
        .arg("debtap")
        .arg("-u")
        .status()
        .map_err(|e| KhazaurError::Io(e))?;

    if !status.success() {
        return Err(KhazaurError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to update debtap database",
        )));
    }

    println!("{}", ui::success("Debtap database updated"));
    Ok(())
}

/// Install a .deb package using debtap
pub async fn install_deb(path: &str) -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config("debtap is not installed. Please install 'debtap' from AUR first.".to_string()));
    }

    let deb_path = Path::new(path);
    if !deb_path.exists() {
        return Err(KhazaurError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path),
        )));
    }

    println!("{}", ui::section_header(&format!("Converting {}", path)));
    
    // Check if debtap database exists (warn but don't auto-update)
    let db_path = Path::new("/var/cache/debtap/pkgfile.txt");
    if !db_path.exists() {
        println!("{}", ui::warning("Debtap database not initialized. Run 'khazaur -Sy' to update it."));
    }
    
    println!("{}", ui::info("Running debtap conversion (this may take a while)..."));

    // Get the directory containing the .deb file
    let search_dir = deb_path.parent().unwrap_or_else(|| Path::new("."));
    
    // Record the start time before running debtap
    let conversion_start = std::time::SystemTime::now();
    
    // Small delay to ensure filesystem timestamp separation
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Run debtap conversion
    // debtap is interactive, so we let it inherit stdin/stdout
    let status = Command::new("debtap")
        .arg(path)
        .status()
        .map_err(|e| KhazaurError::Io(e))?;

    if !status.success() {
        return Err(KhazaurError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Debtap conversion failed",
        )));
    }

    // Find .pkg.tar.zst files created during the conversion
    // (modification time after our start time)
    let mut candidate_packages = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".pkg.tar.zst") {
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            // Check if this file was created/modified after we started
                            if modified >= conversion_start {
                                candidate_packages.push((path, modified));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by modification time (newest first) and take the most recent one
    candidate_packages.sort_by(|a, b| b.1.cmp(&a.1));
    
    if let Some((pkg_path, _)) = candidate_packages.first() {
        println!("\n{}", ui::info(&format!("Found generated package: {}", pkg_path.display())));
        
        // Install with pacman -U
        println!("{}", ui::section_header("Installing Converted Package"));
        crate::pacman::install_local_package(pkg_path.to_str().unwrap(), &Vec::new())?;
        
        return Ok(());
    }

    println!("{}", ui::warning("Could not automatically detect the generated package file."));
    println!("Please install the generated .pkg.tar.zst file manually using 'khazaur -U <file>'");
    
    Ok(())
}
