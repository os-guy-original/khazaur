use crate::config::Config;
use crate::error::{KhazaurError, Result};
use crate::ui;
use std::process::Command;

/// Try to run a command with privilege escalation
/// Tries pkexec first, then sudo, then doas
fn run_privileged(args: &[&str]) -> Result<bool> {
    // Try pkexec first (works with both sudo and doas)
    let mut cmd = Command::new("pkexec");
    cmd.args(args);
    
    if let Ok(status) = cmd.status() {
        return Ok(status.success());
    }
    
    // Try sudo
    let mut cmd = Command::new("sudo");
    cmd.args(args);
    
    if let Ok(status) = cmd.status() {
        return Ok(status.success());
    }
    
    // Try doas
    let mut cmd = Command::new("doas");
    cmd.args(args);
    
    if let Ok(status) = cmd.status() {
        return Ok(status.success());
    }
    
    Err(KhazaurError::Config("No privilege escalation tool found (tried pkexec, sudo, doas)".to_string()))
}

/// Check and prompt for flatpak if not installed and not rejected
pub async fn check_and_prompt_flatpak(config: &mut Config) -> Result<()> {
    // If already available, nothing to do
    if crate::flatpak::is_available() {
        return Ok(());
    }
    
    // Check if user permanently rejected
    if config.rejected_dependencies.flatpak {
        return Ok(());
    }
    
    println!("\n{}", ui::info("Flatpak is not installed"));
    println!("Flatpak allows installing applications from Flathub.");
    println!("Install flatpak to access Flatpak packages.\n");
    
    let choice = dialoguer::Select::new()
        .with_prompt("Install flatpak?")
        .items(&["Install now", "Skip for now", "Never ask again"])
        .default(1)
        .interact_opt()?;
    
    match choice {
        Some(0) => {
            // Install flatpak from official repos using pacman
            println!("{}", ui::info("Installing flatpak..."));
            crate::pacman::install_packages(&vec!["flatpak".to_string()], &Vec::new())?;
            println!("{}", ui::success("Flatpak installed successfully"));
        }
        Some(1) => {
            // Skip for now
            println!("{}", ui::info("Skipping flatpak installation"));
        }
        Some(2) => {
            // Never ask again
            config.rejected_dependencies.flatpak = true;
            config.save()?;
            println!("{}", ui::info("Won't ask about flatpak again"));
        }
        None => {
            // User cancelled
            return Ok(());
        }
        _ => {}
    }
    
    Ok(())
}

/// Check and prompt for snapd if not installed and not rejected
pub async fn check_and_prompt_snapd(config: &mut Config) -> Result<()> {
    // If already available, nothing to do
    if crate::snap::is_available() {
        return Ok(());
    }
    
    // Check if user permanently rejected
    if config.rejected_dependencies.snapd {
        return Ok(());
    }
    
    println!("\n{}", ui::info("Snapd is not installed"));
    println!("Snapd allows installing applications from Snap Store.");
    println!("Install snapd to access Snap packages.\n");
    
    let choice = dialoguer::Select::new()
        .with_prompt("Install snapd?")
        .items(&["Install now (from AUR)", "Skip for now", "Never ask again"])
        .default(1)
        .interact_opt()?;
    
    match choice {
        Some(0) => {
            // Install snapd from AUR (it's not in official repos)
            println!("{}", ui::info("Installing snapd from AUR..."));
            
            let packages = vec!["snapd".to_string()];
            let result = Box::pin(crate::cli::install::install(
                &packages,
                config,
                true, // noconfirm
                true, // only_aur - force AUR since snapd is only in AUR
                false, // only_repos
                false, // only_flatpak
                false, // only_snap
                false, // only_debian
                false, // no_timeout
            )).await;
            
            match result {
                Ok(_) => {
                    println!("{}", ui::success("Snapd installed successfully"));
                    
                    // Enable and start snapd services
                    println!("{}", ui::info("Enabling snapd services..."));
                    
                    if run_privileged(&["systemctl", "enable", "--now", "snapd.socket"])? {
                        println!("{}", ui::success("Snapd socket enabled"));
                    } else {
                        eprintln!("{}", ui::warning("Failed to enable snapd socket"));
                    }
                    
                    // Create the classic snap symlink if it doesn't exist
                    if !std::path::Path::new("/snap").exists() {
                        println!("{}", ui::info("Creating /snap symlink..."));
                        if run_privileged(&["ln", "-s", "/var/lib/snapd/snap", "/snap"])? {
                            println!("{}", ui::success("Snap symlink created"));
                        } else {
                            eprintln!("{}", ui::warning("Failed to create /snap symlink"));
                        }
                    }
                    
                    println!("{}", ui::info("You may need to log out and back in for snap to work properly"));
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Failed to install snapd: {}", e)));
                }
            }
        }
        Some(1) => {
            // Skip for now
            println!("{}", ui::info("Skipping snapd installation"));
        }
        Some(2) => {
            // Never ask again
            config.rejected_dependencies.snapd = true;
            config.save()?;
            println!("{}", ui::info("Won't ask about snapd again"));
        }
        None => {
            // User cancelled
            return Ok(());
        }
        _ => {}
    }
    
    Ok(())
}

/// Check and prompt for debtap with WARNING if not installed and not rejected
pub async fn check_and_prompt_debtap(config: &mut Config) -> Result<()> {
    // If already available, nothing to do
    if crate::debtap::is_available() {
        return Ok(());
    }
    
    // Check if user permanently rejected
    if config.rejected_dependencies.debtap {
        return Ok(());
    }
    
    println!("\n{}", ui::warning("⚠️  Debtap is not installed"));
    println!("{}", ui::warning("WARNING: Debtap can potentially conflict with system packages\n"));
    println!("Debtap converts Debian packages to Arch packages, but this");
    println!("conversion is not always perfect and may cause issues.\n");
    
    let choice = dialoguer::Select::new()
        .with_prompt("Install debtap?")
        .items(&["Install now (from AUR)", "Skip for now", "Never ask again"])
        .default(1)
        .interact_opt()?;
    
    match choice {
        Some(0) => {
            // Install debtap from AUR - use our own install but recursively
            // To avoid infinite recursion, we'll use Box::pin
            println!("{}", ui::info("Installing debtap from AUR..."));
            
            let packages = vec!["debtap".to_string()];
            let result = Box::pin(crate::cli::install::install(
                &packages,
                config,
                true, // noconfirm
                true, // only_aur - force AUR to avoid recursion
                false, // only_repos
                false, // only_flatpak
                false, // only_snap
                false, // only_debian
                false, // no_timeout
            )).await;
            
            match result {
                Ok(_) => {
                    println!("{}", ui::success("Debtap installed successfully"));
                    
                    // Initialize debtap database
                    println!("{}", ui::info("Initializing debtap database..."));
                    if run_privileged(&["debtap", "-u"])? {
                        println!("{}", ui::success("Debtap database initialized"));
                    } else {
                        eprintln!("{}", ui::warning("Failed to initialize debtap database"));
                        println!("{}", ui::info("You can run 'sudo debtap -u' manually later"));
                    }
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Failed to install debtap: {}", e)));
                }
            }
        }
        Some(1) => {
            // Skip for now
            println!("{}", ui::info("Skipping debtap installation"));
        }
        Some(2) => {
            // Never ask again
            config.rejected_dependencies.debtap = true;
            config.save()?;
            println!("{}", ui::info("Won't ask about debtap again"));
        }
        None => {
            // User cancelled
            return Ok(());
        }
        _ => {}
    }
    
    Ok(())
}
