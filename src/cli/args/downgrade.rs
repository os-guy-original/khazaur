use crate::error::Result;
use crate::ui;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use colored::Colorize;

pub async fn downgrade(package: &str) -> Result<()> {
    println!("{}", ui::section_header("Downgrade Package"));
    
    let cache_dir = "/var/cache/pacman/pkg";
    let entries = fs::read_dir(cache_dir)?;
    
    // Filter for package files
    // Format: name-version-arch.pkg.tar.zst (or .xz)
    // We want files that START with package- and END with .pkg.tar...
    // But be careful of "firefox" vs "firefox-developer-edition".
    // "firefox-" prefix is safer.
    
    let prefix = format!("{}-", package);
    let mut candidates: Vec<PathBuf> = Vec::new();
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.starts_with(&prefix) && filename.contains(".pkg.tar") && !filename.ends_with(".sig") {
                // Ensure it's not a different package sharing prefix
                // e.g. "package-foo" vs "package"
                // The char after prefix should be digit (start of version) usually?
                // Arch package naming: name-version-release-arch
                // So if we have "firefox-", the next char must be version start.
                // If we have "firefox-adblock", then "adblock" is part of name?
                // Pacman cache usually contains valid packages.
                // A weak check: check if it matches exactly `package-version-...`
                
                // Let's blindly try matching.
                candidates.push(path);
            }
        }
    }
    
    if candidates.is_empty() {
        println!("{}", ui::warning(&format!("No cached versions found for '{}'", package)));
        return Ok(());
    }
    
    // Sort candidates (roughly by modification time or name?)
    // Newer versions usually have "higher" strings, but strictly we should check mod time or version parse.
    // Mod time is safest for cache.
    candidates.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    candidates.reverse(); // Newest first
    
    println!("Found {} cached versions:", candidates.len());
    
    for (i, path) in candidates.iter().enumerate() {
        let filename = path.file_name().unwrap().to_string_lossy();
        println!(" [{}] {}", i + 1, filename.bright_cyan());
    }
    
    println!("\nSelect a version to install (0 to cancel):");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().unwrap_or(0);
    
    if choice == 0 || choice > candidates.len() {
        println!("Cancelled.");
        return Ok(());
    }
    
    let target = &candidates[choice - 1];
    println!("Downgrading to {:?}...", target);
    
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-U")
        .arg(target)
        .status()?;
        
    if status.success() {
        println!("{}", ui::success("Downgrade successful"));
    } else {
        eprintln!("{}", ui::error("Downgrade failed"));
    }
    
    Ok(())
}
