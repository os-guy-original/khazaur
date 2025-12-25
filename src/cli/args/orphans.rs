use crate::ui;
use crate::error::{KhazaurError, Result};
use std::process::Command;

use dialoguer::{theme::ColorfulTheme, Confirm};

pub fn clean_orphans() -> Result<()> {
    println!("{}", ui::section_header("Cleaning Orphaned Packages"));
    
    // --- Pacman Orphans ---
    println!("{}", ui::info("Checking for pacman orphans (unused dependencies)..."));
    
    // Get list of orphans
    let output = Command::new("pacman")
        .args(["-Qtdq"])
        .output()?;
        
    let orphans_str = String::from_utf8_lossy(&output.stdout);
    let orphans: Vec<&str> = orphans_str.lines().filter(|l| !l.is_empty()).collect();
    
    if orphans.is_empty() {
        println!("{}", ui::success("No pacman orphans found"));
    } else {
        println!("{}", ui::info(&format!("Found {} orphan(s):", orphans.len())));
        for pkg in &orphans {
            println!("  {}", pkg);
        }
        println!();
        
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Remove these packages?")
            .default(false)
            .interact()?;
            
        if confirm {
            let mut args = vec!["-Rns", "--noconfirm"];
            args.extend(orphans);
            
            let status = Command::new("sudo")
                .arg("pacman")
                .args(&args)
                .status()?;
                
            if status.success() {
                println!("{}", ui::success("Orphans removed successfully"));
            } else {
                eprintln!("{}", ui::error("Failed to remove orphans"));
            }
        } else {
            println!("{}", ui::warning("Skipping pacman orphan removal"));
        }
    }
    
    // --- Flatpak Unused ---
    if crate::flatpak::is_available() {
        println!("\n{}", ui::info("Checking for unused Flatpak runtimes..."));
        
        // Flatpak remove --unused
        // We run it with --assumeyes if confirmed, but first let's see if we can list them?
        // simple way: just run flatpak uninstall --unused interactively or verify first.
        // There isn't a clean "list unused" command without parsing. 
        // We'll run `flatpak uninstall --unused` and let it handle interaction if not noconfirm,
        // but since we want to be consistent, we can try to just run it. 
        // However, users prefer to KNOW if there are orphans first.
        
        // We can mimic `flatpak uninstall --unused` roughly, or just invoke it.
        // Let's invoke it directly as it handles its own detection well.
        
        println!("{}", ui::info("Running 'flatpak uninstall --unused'..."));
        let status = Command::new("flatpak")
            .args(["uninstall", "--unused"])
            .status()?;
            
        if status.success() {
            println!("{}", ui::success("Flatpak cleanup complete"));
        } else {
            // It might fail if no unused refs, or user cancelled. 
            // Flatpak exit codes are not always super precise for "nothing to do".
        }
    }

    Ok(())
}
