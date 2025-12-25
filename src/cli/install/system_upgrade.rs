use crate::aur::{download, AurClient, AurPackage};
use crate::build;
use crate::cli::install::version_utils::needs_update;
use crate::config::Config;
use crate::error::Result;
use crate::ui;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm};

/// Upgrade the entire system (repo + AUR + Debian packages)
pub async fn upgrade_system(config: &mut Config, noconfirm: bool) -> Result<()> {
    println!("\n{}", ui::info("Checking for updates..."));

    // Get repository updates
    let repo_updates = crate::pacman::get_repo_updates()?;

    // Get AUR updates
    let installed_aur = crate::pacman::get_installed_aur_packages()?;
    let mut aur_updates = Vec::<(String, String, AurPackage)>::new();

    if !installed_aur.is_empty() {
        let client = AurClient::new()?;
        let package_names: Vec<String> = installed_aur.iter().map(|(name, _)| name.clone()).collect();

        let spinner = ui::spinner("Querying AUR...");
        match client.info_batch(&package_names).await {
            Ok(aur_packages) => {
                spinner.finish_and_clear();

                // Compare versions and find packages that need updates
                for (installed_name, installed_version) in &installed_aur {
                    if let Some(aur_pkg) = aur_packages.iter().find(|p: &&AurPackage| &p.name == installed_name) {
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
    for update in &flatpak_updates {
        println!("  {} {} -> {} {}",
            format!("{} ({})", update.name, update.app_id).bold(),
            update.current_version.dimmed(),
            update.new_version.green(),
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

    // If no updates, show message and return
    if total_updates == 0 {
        println!("\n{}", ui::success("System is up to date"));
        return Ok(());
    }


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
        if let Err(e) = crate::pacman::install_packages(&repo_names, &extra_args) {
            let _ = crate::history::log_action("update", &repo_names, false);
            return Err(e);
        } else {
             let _ = crate::history::log_action("update", &repo_names, true);
        }
    }

    // Upgrade AUR packages
    if !aur_updates.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), "Upgrading AUR packages...".bold());

        let client = AurClient::new()?;
        let aur_pkgs: Vec<AurPackage> = aur_updates.iter().map(|(_, _, pkg): &(_, _, AurPackage)| pkg.clone()).collect();

        // Download all PKGBUILDs
        println!("\n{} {}", "::".bright_blue().bold(), "Downloading PKGBUILDs...".bold());
        let mut package_dirs = Vec::<std::path::PathBuf>::new();

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

            // For upgrades, we'll check if user wants to remove make dependencies
            // but we'll default to not removing them during upgrades to be safe
            let remove_make_deps = if !noconfirm {
                let make_deps_list = pkg.make_depends.join(", ");
                let prompt = format!(
                    "Remove make dependencies ({}) after upgrading {}?",
                    make_deps_list,
                    pkg.name
                );

                let confirmed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(prompt)
                    .default(false)  // Default to false for upgrades
                    .interact()?;

                confirmed
            } else {
                false  // Don't remove make deps during upgrades with noconfirm
            };

            println!("\n{} {}", "::".bright_cyan(), format!("Building {}...", pkg.name).bold());

            match build::build_and_install_with_make_deps_cleanup(pkg_dir, true, pkg, config, remove_make_deps) {
                Ok(_) => {
                    println!("{}", ui::success(&format!("{} upgraded successfully", pkg.name)));
                    let _ = crate::history::log_action("update", &[pkg.name.clone()], true);
                    upgraded_count += 1;
                }
                Err(e) => {
                    let _ = crate::history::log_action("update", &[pkg.name.clone()], false);
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
                            let _ = crate::history::log_action("update", &[name.clone()], true);
                            upgraded_count += 1;
                        }
                        Err(e) => {
                            let _ = crate::history::log_action("update", &[name.clone()], false);
                            eprintln!("{}", ui::error(&format!("Failed to install {}: {}", name, e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = crate::history::log_action("update", &[name.clone()], false);
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
                let _ = crate::history::log_action(
                    "update", 
                    &flatpak_updates.iter().map(|u| format!("{} ({})", u.name, u.app_id)).collect::<Vec<_>>(), 
                    true
                );
            }
            Err(e) => {
                let _ = crate::history::log_action(
                    "update", 
                    &flatpak_updates.iter().map(|u| format!("{} ({})", u.name, u.app_id)).collect::<Vec<_>>(), 
                    false
                );
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
                let _ = crate::history::log_action("update", &snap_updates.iter().map(|(n,_,_)| n.clone()).collect::<Vec<_>>(), true);
            }
            Err(e) => {
                let _ = crate::history::log_action("update", &snap_updates.iter().map(|(n,_,_)| n.clone()).collect::<Vec<_>>(), false);
                eprintln!("{}", ui::error(&format!("Failed to upgrade Snap packages: {}", e)));
            }
        }
    }

    Ok(())
}