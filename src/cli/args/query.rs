use crate::ui;
use crate::pacman;
use crate::error::Result;
use colored::Colorize;

pub fn query_packages() -> Result<()> {
    println!("{}", ui::section_header("Installed Packages"));
    
    // Get pacman packages (repo + AUR)
    let pacman_packages = pacman::get_installed_packages()?;
    let aur_packages = pacman::get_installed_aur_packages()?;
    
    // Create a set of AUR package names for quick lookup
    let aur_names: std::collections::HashSet<String> = aur_packages
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    
    // Separate repo and AUR packages
    let mut repo_packages = Vec::new();
    for (name, version) in &pacman_packages {
        if !aur_names.contains(name) {
            repo_packages.push((name.clone(), version.clone()));
        }
    }
    
    // Get Flatpak packages
    let flatpak_packages = if crate::flatpak::is_available() {
        crate::flatpak::get_installed_flatpaks("")?
    } else {
        Vec::new()
    };
    
    // Get Snap packages
    let snap_packages = if crate::snap::is_available() {
        crate::snap::get_installed_snaps("")?
    } else {
        Vec::new()
    };
    
    // Display summary
    let total = pacman_packages.len() + flatpak_packages.len() + snap_packages.len();
    println!("\n{} Total: {}, Repository: {}, AUR: {}, Flatpak: {}, Snap: {}\n",
        "::".bright_blue().bold(),
        total,
        repo_packages.len(),
        aur_packages.len(),
        flatpak_packages.len(),
        snap_packages.len()
    );
    
    // Display repository packages
    if !repo_packages.is_empty() {
        println!("{} {} ({})", 
            "::".bright_blue().bold(),
            "Repository Packages".bold(),
            repo_packages.len()
        );
        for (name, version) in &repo_packages {
            println!("  {} {}", name, version.dimmed());
        }
        println!();
    }
    
    // Display AUR packages
    if !aur_packages.is_empty() {
        println!("{} {} ({})", 
            "::".bright_cyan().bold(),
            "AUR Packages".bold(),
            aur_packages.len()
        );
        for (name, version) in &aur_packages {
            println!("  {} {}", name, version.dimmed());
        }
        println!();
    }
    
    // Display Flatpak packages
    if !flatpak_packages.is_empty() {
        println!("{} {} ({})", 
            "::".bright_green().bold(),
            "Flatpak Applications".bold(),
            flatpak_packages.len()
        );
        for app_id in &flatpak_packages {
            println!("  {}", app_id);
        }
        println!();
    }
    
    // Display Snap packages
    if !snap_packages.is_empty() {
        println!("{} {} ({})", 
            "::".bright_yellow().bold(),
            "Snap Packages".bold(),
            snap_packages.len()
        );
        for name in &snap_packages {
            println!("  {}", name);
        }
        println!();
    }
    
    Ok(())
}
