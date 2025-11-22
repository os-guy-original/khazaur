use crate::aur::AurPackage;
use crate::pacman::RepoPackage;
use crate::flatpak::FlatpakPackage;
use crate::snap::SnapPackage;
use colored::*;

/// Format a section header
pub fn section_header(title: &str) -> String {
    format!("\n{}\n{}", title.bright_cyan().bold(), "─".repeat(title.len()).bright_black())
}



/// Format package list from AUR
pub fn format_aur_packages(packages: &[AurPackage], show_installed: bool) -> String {
    if packages.is_empty() {
        return "No packages found".dimmed().to_string();
    }

    let mut output = String::new();
    
    for pkg in packages {
        let installed = if show_installed {
            crate::pacman::is_installed(&pkg.name).unwrap_or(false)
        } else {
            false
        };

        let name = if installed {
            format!("{} {}", pkg.name.bright_green(), "[installed]".bright_black())
        } else {
            pkg.name.bright_white().to_string()
        };

        let version = pkg.version.bright_blue();
        let votes = format!("+{}", pkg.num_votes).bright_magenta();
        let popularity = format!("{:.2}%", pkg.popularity * 100.0).bright_cyan();

        output.push_str(&format!(
            "{}/{} {} ({}, {})\n",
            "aur".bright_yellow(),
            name,
            version,
            votes,
            popularity
        ));

        if let Some(desc) = &pkg.description {
            output.push_str(&format!("    - {}\n", desc.dimmed()));
        }
    }

    output
}

/// Format package list from repos
pub fn format_repo_packages(packages: &[RepoPackage]) -> String {
    if packages.is_empty() {
        return "No packages found".dimmed().to_string();
    }

    let mut output = String::new();
    
    for pkg in packages {
        let name = if pkg.installed {
            format!("{} {}", pkg.name.bright_green(), "[installed]".bright_black())
        } else {
            pkg.name.bright_white().to_string()
        };

        let version = pkg.version.bright_blue();
        let repo = pkg.repository.bright_cyan();

        output.push_str(&format!(
            "{}/{} {}\n",
            repo,
            name,
            version
        ));

        output.push_str(&format!("    - {}\n", pkg.description.dimmed()));
    }

    output
}

/// Format package info detail
pub fn format_aur_info(pkg: &AurPackage) -> String {
    let mut output = String::new();
    
    output.push_str(&section_header(&format!("AUR Package: {}", pkg.name)));
    output.push('\n');
    
    output.push_str(&format!("{:<15} {}\n", "Repository:".bold(), "aur".bright_yellow()));
    output.push_str(&format!("{:<15} {}\n", "Name:".bold(), pkg.name.bright_white()));
    output.push_str(&format!("{:<15} {}\n", "Version:".bold(), pkg.version.bright_blue()));
    
    if let Some(desc) = &pkg.description {
        output.push_str(&format!("{:<15} {}\n", "Description:".bold(), desc));
    }
    
    if let Some(url) = &pkg.url {
        output.push_str(&format!("{:<15} {}\n", "URL:".bold(), url.bright_cyan()));
    }
    
    if let Some(maintainer) = &pkg.maintainer {
        output.push_str(&format!("{:<15} {}\n", "Maintainer:".bold(), maintainer.bright_green()));
    }
    
    output.push_str(&format!("{:<15} {}\n", "Votes:".bold(), pkg.num_votes.to_string().bright_magenta()));
    output.push_str(&format!("{:<15} {:.2}%\n", "Popularity:".bold(), (pkg.popularity * 100.0).to_string().bright_cyan()));
    
    if !pkg.depends.is_empty() {
        output.push_str(&format!("{:<15} {}\n", "Depends On:".bold(), pkg.depends.join("  ")));
    }
    
    if !pkg.make_depends.is_empty() {
        output.push_str(&format!("{:<15} {}\n", "Make Depends:".bold(), pkg.make_depends.join("  ")));
    }
    
    if !pkg.opt_depends.is_empty() {
        output.push_str(&format!("{:<15} {}\n", "Optional Deps:".bold(), pkg.opt_depends.join("  ")));
    }
    
    output
}

/// Format flatpak packages
pub fn format_flatpak_packages(packages: &[FlatpakPackage]) -> String {
    if packages.is_empty() {
        return "No packages found".dimmed().to_string();
    }

    let mut output = String::new();
    
    for pkg in packages {
        let name = pkg.name.bright_white();
        let version = pkg.version.bright_blue();
        let app_id = pkg.app_id.bright_cyan();
        
        output.push_str(&format!(
            "{}/{} {}\n",
            "flatpak".bright_magenta(),
            name,
            version
        ));
        
        output.push_str(&format!("    {} {}\n", "ID:".dimmed(), app_id));
        output.push_str(&format!("    - {}\n", pkg.description.dimmed()));
    }

    output
}

/// Format snap packages
pub fn format_snap_packages(packages: &[SnapPackage]) -> String {
    if packages.is_empty() {
        return "No packages found".dimmed().to_string();
    }

    let mut output = String::new();
    
    for pkg in packages {
        let name = pkg.name.bright_white();
        let version = pkg.version.bright_blue();
        let publisher = pkg.publisher.bright_green();
        
        output.push_str(&format!(
            "{}/{} {} ({})\n",
            "snap".bright_magenta(),
            name,
            version,
            publisher
        ));
        
        output.push_str(&format!("    {}\n", pkg.description.dimmed()));
    }

    output
}
