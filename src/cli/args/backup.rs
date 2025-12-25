use crate::error::Result;
use crate::ui;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::Command;

pub fn backup(path: &PathBuf) -> Result<()> {
    println!("{}", ui::section_header("System Backup"));
    
    // Resolve final path
    let mut final_path = path.clone();
    
    // Determine if path is a directory or file
    // If path exists and is a directory, OR if path has no extension (treat as directory)
    let is_dir = if final_path.exists() {
        final_path.is_dir()
    } else {
        // Path doesn't exist - check if it looks like a file (has extension) or directory
        final_path.extension().is_none()
    };
    
    if is_dir {
        // Create the directory if it doesn't exist
        if !final_path.exists() {
            println!("{}", ui::info(&format!("Creating directory: {:?}", final_path)));
            std::fs::create_dir_all(&final_path)?;
        }
        // Append filename
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        final_path = final_path.join(format!("khazaur_backup_{}.txt", timestamp));
    } else {
        // It's a file path - create parent directories if needed
        if let Some(parent) = final_path.parent() {
            if !parent.exists() {
                println!("{}", ui::info(&format!("Creating directory: {:?}", parent)));
                std::fs::create_dir_all(parent)?;
            }
        }
    }
    
    println!("{}", ui::info(&format!("Backing up package list to {:?}", final_path)));

    // Get native packages (explicitly installed)
    let native_out = Command::new("pacman")
        .args(["-Q", "-q", "-e", "-n"]) // Query, quiet, explicit, native
        .output()?;
    
    // Get foreign packages (AUR, explicit)
    let foreign_out = Command::new("pacman")
        .args(["-Q", "-q", "-e", "-m"])
        .output()?;
        
    let native = String::from_utf8_lossy(&native_out.stdout);
    let foreign = String::from_utf8_lossy(&foreign_out.stdout);
    
    let mut file = File::create(&final_path)?;
    
    writeln!(file, "# Khazaur Package Backup")?;
    writeln!(file, "# Created: {}", chrono::Local::now().to_rfc3339())?;
    writeln!(file, "")?;
    
    writeln!(file, "# Native Packages")?;
    for line in native.lines() {
        writeln!(file, "{}", line)?;
    }
    
    writeln!(file, "")?;
    writeln!(file, "# Foreign/AUR Packages")?;
    for line in foreign.lines() {
        writeln!(file, "{}", line)?;
    }
    
    // Flatpak
    if Command::new("which").arg("flatpak").output().map(|o| o.status.success()).unwrap_or(false) {
        println!("{}", ui::info("Backing up Flatpak packages..."));
        let flatpak_out = Command::new("flatpak")
            .args(["list", "--app", "--columns=application"])
            .output()?;
        let flatpaks = String::from_utf8_lossy(&flatpak_out.stdout);
        
        writeln!(file, "")?;
        writeln!(file, "# Flatpak Packages")?;
        for line in flatpaks.lines() {
            writeln!(file, "{}", line)?;
        }
    }

    // Snap
    if Command::new("which").arg("snap").output().map(|o| o.status.success()).unwrap_or(false) {
         println!("{}", ui::info("Backing up Snap packages..."));
         // Snap list output is table, we need first column, skip header
         let snap_out = Command::new("snap")
            .arg("list")
            .output()?;
         let output_str = String::from_utf8_lossy(&snap_out.stdout);
         
         writeln!(file, "")?;
         writeln!(file, "# Snap Packages")?;
         // Skip first line (header)
         for line in output_str.lines().skip(1) {
             if let Some(name) = line.split_whitespace().next() {
                 writeln!(file, "{}", name)?;
             }
         }
    }
    
    println!("{}", ui::success(&format!("Backup created successfully at {:?}", final_path)));
    Ok(())
}

pub async fn restore(path: &PathBuf) -> Result<()> {
    println!("{}", ui::section_header("System Restore"));
    println!("{}", ui::info(&format!("Restoring from {:?}", path)));
    
    if !path.exists() {
         return Err(crate::error::KhazaurError::Config("Backup file not found".into()).into());
    }
    
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let mut repo_packages = Vec::new();
    let mut aur_packages = Vec::new();
    let mut flatpak_packages = Vec::new();
    let mut snap_packages = Vec::new();
    
    // State machine for parsing
    #[derive(PartialEq)]
    enum Section {
        None,
        Repo,
        Aur,
        Flatpak,
        Snap,
    }
    
    let mut current_section = Section::None;
    
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
             continue;
        }
        
        if trimmed.starts_with('#') {
            let lower = trimmed.to_lowercase();
            if lower.contains("flatpak") {
                current_section = Section::Flatpak;
            } else if lower.contains("snap") {
                current_section = Section::Snap;
            } else if lower.contains("native packages") {
                current_section = Section::Repo;
            } else if lower.contains("foreign") || lower.contains("aur") {
                current_section = Section::Aur;
            }
            continue;
        }
        
        match current_section {
            Section::Repo => repo_packages.push(trimmed.to_string()),
            Section::Aur => aur_packages.push(trimmed.to_string()),
            Section::Flatpak => flatpak_packages.push(trimmed.to_string()),
            Section::Snap => snap_packages.push(trimmed.to_string()),
            Section::None => {} // Skip header comments before first section
        }
    }
    
    let mut config = crate::config::Config::load()?;
    
    // 1. Install Repo Packages (Native)
    if !repo_packages.is_empty() {
        println!("\n{}", ui::section_header(&format!("Restoring {} Repository Packages", repo_packages.len())));
        
        // Use pacman directly for native packages, bypassing search
        match crate::pacman::install_packages(&repo_packages, &Vec::new()) {
            Ok(_) => println!("{}", ui::success("Repository packages installed")),
            Err(e) => eprintln!("{}", ui::error(&format!("Failed to install repository packages: {}", e))),
        }
    }

    // 2. Install AUR Packages
    if !aur_packages.is_empty() {
        println!("\n{}", ui::section_header(&format!("Restoring {} AUR Packages", aur_packages.len())));
        
        // Use install_aur_packages directly
        if let Err(e) = crate::cli::install::install_aur_packages(
            &aur_packages,
            &mut config,
            false, // noconfirm (false = ask? backup restore maybe should be interactive or respected global flag? passing false for now)
        ).await {
            eprintln!("{}", ui::error(&format!("Failed to restore AUR packages: {}", e)));
        }
    }

    // 3. Install Flatpak packages
    if !flatpak_packages.is_empty() {
        println!("\n{}", ui::section_header(&format!("Restoring {} Flatpak Packages", flatpak_packages.len())));
        for app_id in &flatpak_packages {
            if let Err(e) = crate::flatpak::install_flatpak(app_id).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", app_id, e)));
            } else {
                 println!("{}", ui::success(&format!("{} installed", app_id)));
            }
        }
    }

    // 4. Install Snap packages
    if !snap_packages.is_empty() {
         println!("\n{}", ui::section_header(&format!("Restoring {} Snap Packages", snap_packages.len())));
         for name in &snap_packages {
            if let Err(e) = crate::snap::install_snap(name).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", name, e)));
            } else {
                println!("{}", ui::success(&format!("{} installed", name)));
            }
         }
    }
    
    if repo_packages.is_empty() && aur_packages.is_empty() && flatpak_packages.is_empty() && snap_packages.is_empty() {
        println!("{}", ui::warning("No packages found in backup file"));
    } else {
        println!("\n{}", ui::success("Restore process completed"));
    }

    Ok(())
}
