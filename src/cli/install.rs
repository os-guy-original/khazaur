use crate::aur::{download, AurClient};
use crate::build;
use crate::cli::{PackageSource, find_package_sources};
use crate::config::Config;
use crate::error::Result;
use crate::flatpak;
use crate::pacman;
use crate::resolver::Resolver;
use crate::snap;
use crate::ui::{self, select_package_source};
use colored::*;
use tracing::{debug, warn};

/// Install packages from AUR, repos, Flatpak, and Snap
pub async fn install(
    packages: &[String],
    config: &mut Config,
    noconfirm: bool,
    only_aur: bool,
    only_repos: bool,
    only_flatpak: bool,
    only_snap: bool,
    only_debian: bool,
    no_timeout: bool,
) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    // Handle .deb files first (before showing "Finding Package(s)" header)
    let mut non_deb_packages = Vec::new();
    
    for pkg_name in packages {
        if pkg_name.ends_with(".deb") {
            // Check and prompt for debtap if needed
            if !crate::debtap::is_available() {
                crate::cli::optional_deps::check_and_prompt_debtap(config).await?;
            }
            
            if crate::debtap::is_available() {
                if let Err(e) = crate::debtap::install_deb(pkg_name).await {
                    eprintln!("{}", ui::error(&format!("Failed to install .deb package {}: {}", pkg_name, e)));
                }
            } else {
                eprintln!("{}", ui::error(&format!("Skipping {}: debtap not available", pkg_name)));
            }
        } else {
            non_deb_packages.push(pkg_name.clone());
        }
    }
    
    // If we only had .deb files, we're done
    if non_deb_packages.is_empty() {
        return Ok(());
    }

    println!("{}", ui::section_header("Finding Package(s)"));
    
    // Determine what sources we'll be searching
    let search_all = !only_aur && !only_repos && !only_flatpak && !only_snap && !only_debian;
    
    // Prompt for optional dependencies BEFORE searching if needed
    if search_all || only_flatpak {
        if !crate::flatpak::is_available() {
            crate::cli::optional_deps::check_and_prompt_flatpak(config).await?;
        }
    }
    
    if search_all || only_snap {
        if !crate::snap::is_available() {
            crate::cli::optional_deps::check_and_prompt_snapd(config).await?;
        }
    }
    
    if search_all || only_debian {
        if !crate::debtap::is_available() {
            crate::cli::optional_deps::check_and_prompt_debtap(config).await?;
        }
    }
    
    let spinner = ui::spinner("Searching for packages...");

    let client = AurClient::new()?;
    
    let mut aur_packages = Vec::new();
    let mut repo_packages = Vec::new();
    let mut flatpak_packages = Vec::new();
    let mut snap_packages = Vec::new();
    let mut debian_packages = Vec::new();

    // Find all non-deb packages
    for pkg_name in &non_deb_packages {

        // Find all possible sources for this package
        let candidates = find_package_sources(
            pkg_name,
            &client,
            config,
            only_aur,
            only_repos,
            only_flatpak,
            only_snap,
            only_debian,
            no_timeout,
        ).await?;
        
        spinner.finish_and_clear();

        let selected_index = if candidates.is_empty() {
            warn!("Package {} not found in any source", pkg_name);
            continue;
        } else if candidates.len() == 1 {
            // Only one source, use it automatically
            0
        } else {
            // Multiple sources, ask user
            match select_package_source(pkg_name, &candidates)? {
                Some(idx) => idx,
                None => {
                    println!("{}", ui::error("Selection cancelled"));
                    return Ok(());
                }
            }
        };

        match &candidates[selected_index].source {
            PackageSource::Repo(pkg) => {
                debug!("{} found in repositories", pkg.name);
                repo_packages.push(pkg.name.clone());
            }
            PackageSource::Aur(pkg) => {
                debug!("{} found in AUR", pkg.name);
                aur_packages.push(pkg.clone());
            }
            PackageSource::Flatpak(pkg) => {
                debug!("{} found in Flatpak", pkg.app_id);
                flatpak_packages.push(pkg.app_id.clone());
            }
            PackageSource::Snap(pkg) => {
                debug!("{} found in Snap", pkg.name);
                snap_packages.push(pkg.name.clone());
            }
            PackageSource::Debian(pkg) => {
                debug!("{} found in Debian", pkg.name);
                debian_packages.push(pkg.clone());
            }
        }
    }

    // Install repository packages first
    if !repo_packages.is_empty() {
        // Filter out already installed packages
        let mut to_install = Vec::new();
        for pkg in &repo_packages {
            if pacman::is_installed(pkg)? {
                println!("{} {} {}", 
                    "::".bright_blue().bold(),
                    pkg.bold(),
                    "is already installed".dimmed()
                );
            } else {
                to_install.push(pkg.clone());
            }
        }
        
        if !to_install.is_empty() {
            println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} repository packages...", to_install.len()).bold());
            pacman::install_packages(&to_install, &Vec::new())?;
        }
    }

    // Install AUR packages
    if !aur_packages.is_empty() {
        // Filter out already installed packages
        let mut to_install = Vec::new();
        for pkg in &aur_packages {
            if pacman::is_installed(&pkg.name)? {
                println!("{} {} {}", 
                    "::".bright_blue().bold(),
                    pkg.name.bold(),
                    "is already installed".dimmed()
                );
            } else {
                to_install.push(pkg.clone());
            }
        }
        
        if to_install.is_empty() {
            // All packages already installed, nothing to do
            return Ok(());
        }
        
        println!("\n{} {}", "::".bright_blue().bold(), format!("Proceeding with installation of {} AUR packages", to_install.len()).bold());
        
        // Resolve dependencies
        let mut resolver = Resolver::new();
        let build_order = resolver.resolve(&to_install, &client).await?;
        
        if !build_order.is_empty() {
            println!("{} {}", "::".bright_blue().bold(), format!("Build order: {}", build_order.join(" -> ")).bold());
        }

        // User already confirmed by running the install command
        // Just show what will be installed

        // Download all PKGBUILDs first (they're small, pre-download for instant viewing)
        println!("\n{} {}", "::".bright_blue().bold(), "Downloading PKGBUILDs...".bold());
        let mut package_dirs = Vec::new();
        
        for pkg in &to_install {
            let spinner = ui::spinner(&format!("Downloading {}...", pkg.name));
            match download::download_package(&client, &pkg.name, config).await {
                Ok(pkg_dir) => {
                    spinner.finish_with_message(format!("✓ {}", pkg.name));
                    package_dirs.push(pkg_dir);
                }
                Err(e) => {
                    spinner.finish_and_clear();
                    return Err(e);
                }
            }
        }

        // Phase 1: Review all PKGBUILDs and collect user decisions
        let mut packages_to_build: Vec<usize> = Vec::new();
        
        if !noconfirm {
            println!("\n{} {}", "::".bright_blue().bold(), "Reviewing PKGBUILDs...".bold());
            
            for (idx, pkg) in to_install.iter().enumerate() {
                println!("\n{} {} {}", 
                    "::".bright_blue().bold(), 
                    format!("({}/{})", idx + 1, to_install.len()).bright_black(),
                    format!("Review {}...", pkg.name).bold()
                );
                
                let pkgbuild_path = package_dirs[idx].join("PKGBUILD");
                let should_continue = ui::view_pkgbuild_interactive(&pkgbuild_path, config)?;
                if should_continue {
                    packages_to_build.push(idx);
                } else {
                    println!("{} {}", "::".yellow().bold(), format!("Skipping {}", pkg.name).bold());
                }
            }
            
            // Show summary of what will be built
            if !packages_to_build.is_empty() {
                let packages_list: Vec<&str> = packages_to_build.iter()
                    .map(|&idx| to_install[idx].name.as_str())
                    .collect();
                println!("\n{} {}: {}", 
                    "::".bright_blue().bold(), 
                    format!("Packages to build ({})", packages_to_build.len()).bold(),
                    packages_list.join(", ")
                );
            } else {
                println!("\n{} {}", "::".yellow().bold(), "No packages selected for installation".bold());
                return Ok(());
            }
        } else {
            // If noconfirm, build all packages
            packages_to_build = (0..to_install.len()).collect();
        }

        // Phase 2: Build and install packages
        let mut installed_count = 0;
        if !packages_to_build.is_empty() {
            println!("\n{} {}", "::".bright_blue().bold(), "Building packages...".bold());
            
            for &idx in &packages_to_build {
                let pkg = &to_install[idx];
                let pkg_dir = &package_dirs[idx];
                
                println!("\n{} {}", "::".bright_cyan(), format!("Building {}...", pkg.name).bold());
                
                // Build and install with makepkg
                match build::build_and_install(pkg_dir, true) {
                    Ok(_) => {
                        println!("{}", ui::success(&format!("{} installed successfully", pkg.name)));
                        installed_count += 1;
                    }
                    Err(e) => {
                        eprintln!("{}", ui::error(&format!("Build failed for {}: {}", pkg.name, e)));
                    }
                }
            }
        }
        
        // Only show completion if at least one package was installed
        if installed_count > 0 {
            println!("\n{} {}", "::".bright_green().bold(), 
                format!("Successfully installed {} package(s)", installed_count).bold());
        }
    }

    // Install Flatpak packages
    if !flatpak_packages.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} Flatpak packages...", flatpak_packages.len()).bold());
        for app_id in flatpak_packages {
            if let Err(e) = flatpak::install_flatpak(&app_id).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", app_id, e)));
            }
        }
    }

    // Install Snap packages
    if !snap_packages.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} Snap packages...", snap_packages.len()).bold());
        for name in snap_packages {
            if let Err(e) = snap::install_snap(&name).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", name, e)));
            }
        }
    }
    
    // Install Debian packages (download and convert with debtap)
    if !debian_packages.is_empty() {
        // Check and prompt for debtap if needed
        if !crate::debtap::is_available() {
            crate::cli::optional_deps::check_and_prompt_debtap(config).await?;
        }
        
        if crate::debtap::is_available() {
            println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} Debian packages...", debian_packages.len()).bold());
            for pkg in debian_packages {
                // Download .deb file
                match crate::debian::download_debian(&pkg).await {
                    Ok(deb_path) => {
                        // Convert and install with debtap
                        if let Err(e) = crate::debtap::install_deb(deb_path.to_str().unwrap()).await {
                            eprintln!("{}", ui::error(&format!("Failed to install {}: {}", pkg.name, e)));
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", ui::error(&format!("Failed to download {}: {}", pkg.name, e)));
                    }
                }
            }
        } else {
            eprintln!("{}", ui::warning("Skipping Debian packages: debtap not available"));
        }
    }
    
    Ok(())
}

/// Upgrade the entire system
pub async fn upgrade_system(_config: &mut Config, _noconfirm: bool) -> Result<()> {
    println!("{}", ui::section_header("System Upgrade"));

    // First upgrade repo packages
    println!("{}", ui::info("Upgrading repository packages..."));
    pacman::upgrade_system(&Vec::new())?;

    // TODO: Upgrade AUR packages
    // This would require:
    // 1. List all installed AUR packages
    // 2. Check for updates
    // 3. Rebuild and reinstall updated packages
    
    println!("\n{}", ui::info("AUR package upgrades not yet implemented"));
    println!("{}", ui::success("System upgrade complete"));
    
    Ok(())
}
