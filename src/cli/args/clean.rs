use crate::ui;
use crate::error::Result;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::process::Command;


pub fn clean_cache(clean_level: u8) -> Result<()> {
    println!("{}", ui::section_header("Cleaning Package Cache"));
    
    // Get khazaur cache directory
    let cache_dir = crate::dirs::cache_dir()?;
    let clone_dir = cache_dir.join("clone");
    
    // -cc: Clean pacman cache first
    if clean_level >= 2 {
        println!("\n{}", ui::info("Cleaning pacman package cache..."));
        
        let confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Clean pacman cache (/var/cache/pacman/pkg/)?")
            .default(true)
            .interact()?;
        
        if confirm {
            let status = Command::new("sudo")
                .args(["pacman", "-Sc", "--noconfirm"])
                .status();
            
            match status {
                Ok(s) if s.success() => {
                    println!("{}", ui::success("Pacman cache cleaned"));
                }
                Ok(_) => {
                    eprintln!("{}", ui::warning("Failed to clean pacman cache (may require sudo)"));
                }
                Err(e) => {
                    eprintln!("{}", ui::warning(&format!("Failed to run pacman -Sc: {}", e)));
                }
            }
        } else {
            println!("{}", ui::info("Skipping pacman cache"));
        }
    }
    
    // -c or -cc: Clean khazaur AUR cache with per-folder confirmation
    println!("\n{}", ui::info("Cleaning khazaur AUR cache..."));
    
    if clone_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&clone_dir)?
            .filter_map(|e| e.ok())
            .collect();
        
        if entries.is_empty() {
            println!("{}", ui::info("Khazaur cache is already empty"));
        } else {
            println!("{}", ui::info(&format!("Found {} cached AUR package(s)", entries.len())));
            
            let mut removed = 0;
            let mut skipped = 0;
            
            for entry in entries {
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                
                // Calculate size
                let size = dir_size(&path).unwrap_or(0);
                let size_str = format_size(size);
                
                let confirm = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!("Remove '{}' ({})?", name, size_str))
                    .default(true)
                    .interact()?;
                
                if confirm {
                    match std::fs::remove_dir_all(&path) {
                        Ok(_) => {
                            println!("{}", ui::success(&format!("Removed: {}", name)));
                            removed += 1;
                        }
                        Err(e) => {
                            eprintln!("{}", ui::warning(&format!(
                                "Failed to remove '{}': {}. Try: sudo rm -rf {:?}",
                                name, e, path
                            )));
                        }
                    }
                } else {
                    skipped += 1;
                }
            }
            
            println!("\n{}", ui::info(&format!("Removed: {}, Skipped: {}", removed, skipped)));
        }
    } else {
        println!("{}", ui::info("Khazaur cache directory does not exist"));
    }
    
    println!("\n{}", ui::success("Cache cleaning complete"));
    Ok(())
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += dir_size(&path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    } else {
        size = std::fs::metadata(path)?.len();
    }
    Ok(size)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
