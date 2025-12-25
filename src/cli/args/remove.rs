use crate::ui;
use crate::pacman;
use crate::error::Result;

use dialoguer::{theme::ColorfulTheme, MultiSelect, Confirm};

/// Remove packages
pub fn remove_packages(packages: &[String]) -> Result<()> {
    println!("{}", ui::section_header("Removing Packages"));
    
    let mut pacman_packages = Vec::new();
    let mut flatpak_packages = Vec::new();
    let mut snap_packages = Vec::new();
    
    for query in packages {
        // Search across all sources
        let pacman_matches = pacman::search_installed_packages(query)?;
        let flatpak_matches = if crate::flatpak::is_available() {
            crate::flatpak::get_installed_flatpaks(query)?
        } else {
            Vec::new()
        };
        let snap_matches = if crate::snap::is_available() {
            crate::snap::get_installed_snaps(query)?
        } else {
            Vec::new()
        };
        
        let total_matches = pacman_matches.len() + flatpak_matches.len() + snap_matches.len();
        
        if total_matches == 0 {
            println!("{}", ui::warning(&format!("No installed packages found matching '{}'", query)));
            continue;
        } else if total_matches == 1 {
            // Single match, add directly
            if !pacman_matches.is_empty() {
                println!("{}", ui::info(&format!("Found (pacman): {}", pacman_matches[0])));
                pacman_packages.push(pacman_matches[0].clone());
            } else if !flatpak_matches.is_empty() {
                println!("{}", ui::info(&format!("Found (flatpak): {}", flatpak_matches[0])));
                flatpak_packages.push(flatpak_matches[0].clone());
            } else {
                println!("{}", ui::info(&format!("Found (snap): {}", snap_matches[0])));
                snap_packages.push(snap_matches[0].clone());
            }
        } else {
            // Multiple matches, show selection UI with source indicators
            
            println!("\n{}", ui::info(&format!("Multiple packages found matching '{}':", query)));
            
            // Build items with source indicators
            let mut items = Vec::new();
            let mut sources = Vec::new();
            
            for pkg in &pacman_matches {
                items.push(format!("{} (pacman)", pkg));
                sources.push(("pacman", pkg.clone()));
            }
            for pkg in &flatpak_matches {
                items.push(format!("{} (flatpak)", pkg));
                sources.push(("flatpak", pkg.clone()));
            }
            for pkg in &snap_matches {
                items.push(format!("{} (snap)", pkg));
                sources.push(("snap", pkg.clone()));
            }
            
            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select packages to remove (Space to select, Enter to confirm)")
                .items(&items)
                .max_length(15)
                .interact()?;
            
            if selections.is_empty() {
                println!("{}", ui::warning("No packages selected"));
                continue;
            }
            
            for &idx in &selections {
                let (source, pkg) = &sources[idx];
                match *source {
                    "pacman" => pacman_packages.push(pkg.clone()),
                    "flatpak" => flatpak_packages.push(pkg.clone()),
                    "snap" => snap_packages.push(pkg.clone()),
                    _ => {}
                }
            }
        }
    }
    
    let total_to_remove = pacman_packages.len() + flatpak_packages.len() + snap_packages.len();
    
    if total_to_remove == 0 {
        println!("{}", ui::warning("No packages to remove"));
        return Ok(());
    }
    
    // Show summary
    if !pacman_packages.is_empty() {
        println!("\n{}", ui::info(&format!("Pacman packages: {}", pacman_packages.join(", "))));
    }
    if !flatpak_packages.is_empty() {
        println!("{}", ui::info(&format!("Flatpak apps: {}", flatpak_packages.join(", "))));
    }
    if !snap_packages.is_empty() {
        println!("{}", ui::info(&format!("Snap packages: {}", snap_packages.join(", "))));
    }
    
    // Ask for confirmation
    
    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed with removal?")
        .default(false)
        .interact()?;
    
    if !confirmed {
        println!("{}", ui::warning("Removal cancelled"));
        return Ok(());
    }
    
    // Remove pacman packages (pass --noconfirm since user already confirmed)
    if !pacman_packages.is_empty() {
        match pacman::remove_packages(&pacman_packages, &vec!["--noconfirm".to_string()]) {
            Ok(_) => {
                println!("{}", ui::success("Pacman packages removed successfully"));
                let _ = crate::history::log_action("remove", &pacman_packages, true);
            },
            Err(e) => {
                let _ = crate::history::log_action("remove", &pacman_packages, false);
                let error_msg = e.to_string();
                
                // Check if it's a dependency conflict
                if error_msg.contains("dependency_conflict:") {
                    println!("\n{}", ui::warning("⚠️  Dependency conflict detected"));
                    println!("{}", ui::info("Some packages depend on the package(s) you're trying to remove."));
                    println!("{}", ui::info("You can force removal with -Rdd, but this may break dependent packages.\n"));
                    
                    
                    let force_remove = Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("Force removal (skip dependency checks)?")
                        .default(false)
                        .interact()?;
                    
                    if force_remove {
                        println!("{}", ui::warning("Force removing packages (ignoring dependencies)..."));
                        pacman::remove_packages(&pacman_packages, &vec!["-dd".to_string(), "--noconfirm".to_string()])?;
                    } else {
                        println!("{}", ui::warning("Removal cancelled"));
                        return Ok(());
                    }
                } else {
                    return Err(e);
                }
            }
        }
    }
    
    // Remove flatpak packages
    for app_id in &flatpak_packages {
        if let Err(e) = crate::flatpak::uninstall_flatpak(app_id) {
            eprintln!("{}", ui::error(&format!("Failed to remove flatpak {}: {}", app_id, e)));
            let _ = crate::history::log_action("remove", &[app_id.clone()], false);
        } else {
            println!("{}", ui::success(&format!("Removed flatpak: {}", app_id)));
            let _ = crate::history::log_action("remove", &[app_id.clone()], true);
        }
    }
    
    // Remove snap packages
    for pkg in &snap_packages {
        if let Err(e) = crate::snap::uninstall_snap(pkg) {
            eprintln!("{}", ui::error(&format!("Failed to remove snap {}: {}", pkg, e)));
            let _ = crate::history::log_action("remove", &[pkg.clone()], false);
        } else {
            println!("{}", ui::success(&format!("Removed snap: {}", pkg)));
            let _ = crate::history::log_action("remove", &[pkg.clone()], true);
        }
    }
    
    println!("\n{}", ui::success("Package removal complete"));
    Ok(())
}
