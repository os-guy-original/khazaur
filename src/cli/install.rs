use crate::aur::{download, AurClient, AurPackage};
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

/// Install packages from AUR, repos, Flatpak, Snap and Debian
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

    // Parse packages and handle source prefixes (e.g., aur/package, repo/package)
    let mut parsed_packages = Vec::new();
    let mut deb_files = Vec::new();
    
    for pkg_name in packages {
        if pkg_name.ends_with(".deb") {
            deb_files.push(pkg_name.clone());
        } else if pkg_name.contains('/') {
            // Parse source prefix (e.g., aur/package, core/package, flatpak/app)
            let parts: Vec<&str> = pkg_name.splitn(2, '/').collect();
            if parts.len() == 2 {
                let source = parts[0];
                let name = parts[1].to_string();
                parsed_packages.push((name, Some(source.to_string())));
            } else {
                parsed_packages.push((pkg_name.clone(), None));
            }
        } else {
            parsed_packages.push((pkg_name.clone(), None));
        }
    }
    
    // Handle .deb files first
    for deb_file in deb_files {
        // Check and prompt for debtap if needed
        if !crate::debtap::is_available() {
            crate::cli::optional_deps::check_and_prompt_debtap(config).await?;
        }
        
        if crate::debtap::is_available() {
            match crate::debtap::install_deb(&deb_file).await {
                Ok(_) => {
                    // Try to extract package name from .deb file and track it
                    // This is best-effort, may not always work
                    if let Some(pkg_name) = std::path::Path::new(&deb_file)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .and_then(|s| s.split('_').next())
                    {
                        let _ = crate::debian::track_debian_package(pkg_name);
                    }
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Failed to install .deb package {}: {}", deb_file, e)));
                }
            }
        } else {
            eprintln!("{}", ui::error(&format!("Skipping {}: debtap not available", deb_file)));
        }
    }
    
    // If we only had .deb files, we're done
    if parsed_packages.is_empty() {
        return Ok(());
    }

    println!("{}", ui::section_header("Finding Package(s)"));
    
    // Determine what sources we'll be searching (considering explicit source prefixes)
    let has_explicit_sources = parsed_packages.iter().any(|(_, src)| src.is_some());
    let search_all = !only_aur && !only_repos && !only_flatpak && !only_snap && !only_debian && !has_explicit_sources;
    
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
    
    let spinner = ui::Spinner::new("Searching for packages...");

    let client = AurClient::new()?;
    
    let mut aur_packages = Vec::new();
    let mut repo_packages = Vec::new();
    let mut flatpak_packages = Vec::new();
    let mut snap_packages = Vec::new();
    let mut debian_packages = Vec::new();
    
    // First, search for all packages
    let mut all_candidates = Vec::new();
    
    for (pkg_name, explicit_source) in &parsed_packages {
        // Determine search flags based on explicit source or command flags
        let (search_aur, search_repos, search_flatpak, search_snap, search_debian) = 
            if let Some(source) = explicit_source {
                // Explicit source specified (e.g., aur/package)
                match source.to_lowercase().as_str() {
                    "aur" => (true, false, false, false, false),
                    "repo" | "core" | "extra" | "multilib" | "community" => (false, true, false, false, false),
                    "flatpak" => (false, false, true, false, false),
                    "snap" => (false, false, false, true, false),
                    "debian" => (false, false, false, false, true),
                    _ => {
                        // Unknown source, treat as repo name and search repos
                        (false, true, false, false, false)
                    }
                }
            } else {
                // No explicit source, use command flags or search all
                (
                    only_aur || search_all,
                    only_repos || search_all,
                    only_flatpak || search_all,
                    only_snap || search_all,
                    only_debian || search_all,
                )
            };

        // Check cache first
        let candidates = if let Some(cached) = crate::cache::get_cached_search(pkg_name) {
            spinner.inner().set_message(format!("Found '{}' in cache - {} sources", pkg_name, cached.len()));
            cached
        } else {
            // Find all possible sources for this package
            let found = find_package_sources(
                pkg_name,
                &client,
                config,
                search_aur,
                search_repos,
                search_flatpak,
                search_snap,
                search_debian,
                no_timeout,
                Some(spinner.inner()),
            ).await?;
            
            // Cache the results
            let _ = crate::cache::cache_search_results(pkg_name.clone(), found.clone());
            found
        };
        
        all_candidates.push((pkg_name.clone(), explicit_source.clone(), candidates));
    }
    
    // Clear spinner after all searches complete
    spinner.inner().finish_and_clear();
    
    // Now process all candidates and ask for selections
    for (pkg_name, explicit_source, candidates) in all_candidates {
        let selected_index = if candidates.is_empty() {
            if explicit_source.is_some() {
                warn!("Package {} not found in {}", pkg_name, explicit_source.as_ref().unwrap());
            } else {
                warn!("Package {} not found in any source", pkg_name);
            }
            continue;
        } else if candidates.len() == 1 || explicit_source.is_some() {
            // Only one source, or explicit source specified - use it automatically
            0
        } else {
            // Multiple sources, ask user
            match select_package_source(&pkg_name, &candidates)? {
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
            if let Err(e) = pacman::install_packages(&to_install, &Vec::new()) {
                eprintln!("{}", ui::error(&format!("Failed to install repository packages: {}", e)));
                eprintln!("{}", ui::info("Continuing with other packages..."));
            }
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
                    eprintln!("{}", ui::error(&format!("Failed to download {}: {}", pkg.name, e)));
                    eprintln!("{}", ui::info("Continuing with other packages..."));
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
                        } else {
                            // Track this package as installed from Debian
                            let _ = crate::debian::track_debian_package(&pkg.name);
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

/// Upgrade the entire system (repo + AUR + Debian packages)
pub async fn upgrade_system(config: &mut Config, noconfirm: bool) -> Result<()> {
    println!("\n{}", ui::info("Checking for updates..."));
    
    // Get repository updates
    let repo_updates = pacman::get_repo_updates()?;
    
    // Get AUR updates
    let installed_aur = pacman::get_installed_aur_packages()?;
    let mut aur_updates = Vec::new();
    
    if !installed_aur.is_empty() {
        let client = AurClient::new()?;
        let package_names: Vec<String> = installed_aur.iter().map(|(name, _)| name.clone()).collect();
        
        let spinner = ui::spinner("Querying AUR...");
        match client.info_batch(&package_names).await {
            Ok(aur_packages) => {
                spinner.finish_and_clear();
                
                // Compare versions and find packages that need updates
                for (installed_name, installed_version) in &installed_aur {
                    if let Some(aur_pkg) = aur_packages.iter().find(|p| &p.name == installed_name) {
                        if needs_update(installed_version, &aur_pkg.version)? {
                            aur_updates.push((installed_name.clone(), installed_version.clone(), aur_pkg.clone()));
                        }
                    }
                }
            }
            Err(e) => {
                spinner.finish_and_clear();
                eprintln!("{}", ui::warning(&format!("Failed to query AUR: {}", e)));
            }
        }
    }
    
    // Get Debian updates (if debtap is available)
    let mut debian_updates = Vec::new();
    if crate::debtap::is_available() {
        let spinner = ui::spinner("Checking Debian packages...");
        match crate::debian::check_debian_updates().await {
            Ok(updates) => {
                spinner.finish_and_clear();
                debian_updates = updates;
            }
            Err(e) => {
                spinner.finish_and_clear();
                eprintln!("{}", ui::warning(&format!("Failed to check Debian updates: {}", e)));
            }
        }
    }
    
    // Check for Flatpak updates
    let flatpak_updates = if crate::flatpak::is_available() {
        let spinner = ui::spinner("Checking Flatpak packages...");
        let updates = crate::flatpak::get_updates().unwrap_or_default();
        spinner.finish_and_clear();
        updates
    } else {
        Vec::new()
    };
    
    // Check for Snap updates
    let snap_updates = if crate::snap::is_available() {
        let spinner = ui::spinner("Checking Snap packages...");
        let updates = crate::snap::get_updates().unwrap_or_default();
        spinner.finish_and_clear();
        updates
    } else {
        Vec::new()
    };
    
    // Show all available updates in unified format
    let total_updates = repo_updates.len() + aur_updates.len() + debian_updates.len() + flatpak_updates.len() + snap_updates.len();
    let has_other_updates = !flatpak_updates.is_empty() || !snap_updates.is_empty();
    
    if total_updates == 0 && !has_other_updates {
        println!("{}", ui::success("System is up to date"));
        return Ok(());
    }
    
    println!("\n{} {}", "::".bright_blue().bold(), format!("Packages ({}):", total_updates).bold());
    
    // Show repo updates
    for (name, old_ver, new_ver) in &repo_updates {
        println!("  {} {} -> {}", 
            name.bold(),
            old_ver.dimmed(),
            new_ver.green()
        );
    }
    
    // Show AUR updates
    for (name, old_ver, aur_pkg) in &aur_updates {
        println!("  {} {} -> {} {}", 
            name.bold(),
            old_ver.dimmed(),
            aur_pkg.version.green(),
            "[AUR]".bright_cyan()
        );
    }
    
    // Show Debian updates
    for (name, old_ver, new_ver, _) in &debian_updates {
        println!("  {} {} -> {} {}", 
            name.bold(),
            old_ver.dimmed(),
            new_ver.green(),
            "[Debian]".bright_magenta()
        );
    }
    
    // Show Flatpak updates
    for (name, old_ver, new_ver) in &flatpak_updates {
        println!("  {} {} -> {} {}", 
            name.bold(),
            old_ver.dimmed(),
            new_ver.green(),
            "[Flatpak]".bright_yellow()
        );
    }
    
    // Show Snap updates
    for (name, old_ver, new_ver) in &snap_updates {
        println!("  {} {} -> {} {}", 
            name.bold(),
            old_ver.dimmed(),
            new_ver.green(),
            "[Snap]".bright_yellow()
        );
    }
    
    // Calculate download size for repo packages (if possible)
    println!("\n{} Repository: {}, AUR: {}, Flatpak: {}, Snap: {}, Debian: {}", 
        "::".bright_blue().bold(),
        repo_updates.len(),
        aur_updates.len(),
        flatpak_updates.len(),
        snap_updates.len(),
        debian_updates.len()
    );
    
    // Ask for confirmation unless noconfirm is set
    if !noconfirm {
        use dialoguer::{theme::ColorfulTheme, Confirm};
        
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with upgrade?")
            .default(true)
            .interact()?;
        
        if !confirmed {
            println!("{}", ui::warning("Upgrade cancelled"));
            return Ok(());
        }
    }
    
    // Upgrade repository packages first
    if !repo_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading repository packages...".bold());
        let repo_names: Vec<String> = repo_updates.iter().map(|(name, _, _)| name.clone()).collect();
        let extra_args = if noconfirm { vec!["--noconfirm".to_string()] } else { vec![] };
        pacman::install_packages(&repo_names, &extra_args)?;
    }
    
    // Upgrade AUR packages
    if !aur_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading AUR packages...".bold());
        
        let client = AurClient::new()?;
        let aur_pkgs: Vec<AurPackage> = aur_updates.iter().map(|(_, _, pkg)| pkg.clone()).collect();
        
        // Download all PKGBUILDs
        println!("\n{} {}", "::".bright_blue().bold(), "Downloading PKGBUILDs...".bold());
        let mut package_dirs = Vec::new();
        
        for pkg in &aur_pkgs {
            let spinner = ui::spinner(&format!("Downloading {}...", pkg.name));
            match download::download_package(&client, &pkg.name, config).await {
                Ok(pkg_dir) => {
                    spinner.finish_with_message(format!("✓ {}", pkg.name));
                    package_dirs.push(pkg_dir);
                }
                Err(e) => {
                    spinner.finish_and_clear();
                    eprintln!("{}", ui::error(&format!("Failed to download {}: {}", pkg.name, e)));
                    continue;
                }
            }
        }
        
        // Review PKGBUILDs if not noconfirm
        let mut packages_to_build: Vec<usize> = Vec::new();
        
        if !noconfirm && config.review_pkgbuild {
            println!("\n{} {}", "::".bright_blue().bold(), "Reviewing PKGBUILDs...".bold());
            
            for (idx, pkg) in aur_pkgs.iter().enumerate() {
                if idx >= package_dirs.len() {
                    continue;
                }
                
                println!("\n{} {} {}", 
                    "::".bright_blue().bold(), 
                    format!("({}/{})", idx + 1, aur_pkgs.len()).bright_black(),
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
            
            if packages_to_build.is_empty() {
                println!("\n{} {}", "::".yellow().bold(), "No AUR packages selected for upgrade".bold());
                return Ok(());
            }
        } else {
            // Build all packages
            packages_to_build = (0..aur_pkgs.len().min(package_dirs.len())).collect();
        }
        
        // Build and install packages
        println!("\n{} {}", "::".bright_blue().bold(), "Building and installing AUR packages...".bold());
        let mut upgraded_count = 0;
        
        for &idx in &packages_to_build {
            let pkg = &aur_pkgs[idx];
            let pkg_dir = &package_dirs[idx];
            
            println!("\n{} {}", "::".bright_cyan(), format!("Building {}...", pkg.name).bold());
            
            match build::build_and_install(pkg_dir, true) {
                Ok(_) => {
                    println!("{}", ui::success(&format!("{} upgraded successfully", pkg.name)));
                    upgraded_count += 1;
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Build failed for {}: {}", pkg.name, e)));
                }
            }
        }
        
        if upgraded_count > 0 {
            println!("\n{} {}", 
                "::".bright_green().bold(), 
                format!("Successfully upgraded {} AUR package(s)", upgraded_count).bold()
            );
        }
    }
    
    // Upgrade Debian packages
    if !debian_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading Debian packages...".bold());
        
        let mut upgraded_count = 0;
        
        for (name, _, _, debian_pkg) in &debian_updates {
            println!("\n{} {}", "::".bright_cyan(), format!("Downloading and converting {}...", name).bold());
            
            // Download .deb file
            match crate::debian::download_debian(debian_pkg).await {
                Ok(deb_path) => {
                    // Convert and install with debtap
                    match crate::debtap::install_deb(deb_path.to_str().unwrap()).await {
                        Ok(_) => {
                            // Track this package as installed from Debian
                            let _ = crate::debian::track_debian_package(name);
                            println!("{}", ui::success(&format!("{} upgraded successfully", name)));
                            upgraded_count += 1;
                        }
                        Err(e) => {
                            eprintln!("{}", ui::error(&format!("Failed to install {}: {}", name, e)));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Failed to download {}: {}", name, e)));
                }
            }
        }
        
        if upgraded_count > 0 {
            println!("\n{} {}", 
                "::".bright_green().bold(), 
                format!("Successfully upgraded {} Debian package(s)", upgraded_count).bold()
            );
        }
    }
    
    // Upgrade Flatpak packages
    if !flatpak_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading Flatpak packages...".bold());
        match crate::flatpak::update_all() {
            Ok(_) => {
                println!("{}", ui::success(&format!("Successfully upgraded {} Flatpak package(s)", flatpak_updates.len())));
            }
            Err(e) => {
                eprintln!("{}", ui::error(&format!("Failed to upgrade Flatpak packages: {}", e)));
            }
        }
    }
    
    // Upgrade Snap packages
    if !snap_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading Snap packages...".bold());
        println!("{}", ui::warning("Note: Snap update support is experimental and not fully tested"));
        match crate::snap::update_all() {
            Ok(_) => {
                println!("{}", ui::success(&format!("Successfully upgraded {} Snap package(s)", snap_updates.len())));
            }
            Err(e) => {
                eprintln!("{}", ui::error(&format!("Failed to upgrade Snap packages: {}", e)));
            }
        }
    }
    
    Ok(())
}

/// Check if a package needs an update by comparing versions
fn needs_update(installed_version: &str, aur_version: &str) -> Result<bool> {
    use std::process::Command;
    
    let output = Command::new("vercmp")
        .arg(installed_version)
        .arg(aur_version)
        .output()?;
    
    if !output.status.success() {
        return Ok(false);
    }
    
    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // vercmp returns:
    // -1 if installed < aur (update needed)
    //  0 if installed == aur (no update)
    //  1 if installed > aur (downgrade, no update)
    Ok(result == "-1")
}
