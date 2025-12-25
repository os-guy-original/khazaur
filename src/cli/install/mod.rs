mod aur_install;
mod system_upgrade;
mod version_utils;

pub use aur_install::*;
pub use system_upgrade::*;

use crate::aur::AurClient;
use crate::config::Config;
use crate::error::Result;
use crate::ui;
use colored::*;

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
                        let _ = crate::history::log_action("install", &[pkg_name.to_string()], true);
                    }
                }
                Err(e) => {
                    let _ = crate::history::log_action("install", &[deb_file.clone()], false);
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

        // Find all possible sources for this package
        let candidates = crate::cli::find_package_sources(
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

        all_candidates.push((pkg_name.clone(), explicit_source.clone(), candidates));
    }

    // Clear spinner after all searches complete
    spinner.inner().finish_and_clear();

    // Now process all candidates and ask for selections
    for (pkg_name, explicit_source, candidates) in all_candidates {
        let selected_index = if candidates.is_empty() {
            if explicit_source.is_some() {
                tracing::warn!("Package {} not found in {}", pkg_name, explicit_source.as_ref().unwrap());
            } else {
                tracing::warn!("Package {} not found in any source", pkg_name);
            }
            continue;
        } else if candidates.len() == 1 || explicit_source.is_some() {
            // Only one source, or explicit source specified - use it automatically
            0
        } else {
            // Multiple sources, ask user
            match crate::ui::select_package_source(&pkg_name, &candidates)? {
                Some(idx) => idx,
                None => {
                    println!("{}", ui::error("Selection cancelled"));
                    return Ok(());
                }
            }
        };

        match &candidates[selected_index].source {
            crate::cli::PackageSource::Repo(pkg) => {
                tracing::debug!("{} found in repositories", pkg.name);
                repo_packages.push(pkg.name.clone());
            }
            crate::cli::PackageSource::Aur(pkg) => {
                tracing::debug!("{} found in AUR", pkg.name);
                aur_packages.push(pkg.clone());
            }
            crate::cli::PackageSource::Flatpak(pkg) => {
                tracing::debug!("{} found in Flatpak", pkg.app_id);
                flatpak_packages.push(pkg.app_id.clone());
            }
            crate::cli::PackageSource::Snap(pkg) => {
                tracing::debug!("{} found in Snap", pkg.name);
                snap_packages.push(pkg.name.clone());
            }
            crate::cli::PackageSource::Debian(pkg) => {
                tracing::debug!("{} found in Debian", pkg.name);
                debian_packages.push(pkg.clone());
            }
        }
    }

    // Install repository packages first
    if !repo_packages.is_empty() {
        // Filter out already installed packages
        let mut to_install = Vec::new();
        for pkg in &repo_packages {
            if crate::pacman::is_installed(pkg)? {
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
            if let Err(e) = crate::pacman::install_packages(&to_install, &Vec::new()) {
                let _ = crate::history::log_action("install", &to_install, false);
                eprintln!("{}", ui::error(&format!("Failed to install repository packages: {}", e)));
                eprintln!("{}", ui::info("Continuing with other packages..."));
            } else {
                let _ = crate::history::log_action("install", &to_install, true);
            }
        }
    }

    // Install AUR packages
    if !aur_packages.is_empty() {
        // Filter out already installed packages
        let mut to_install = Vec::new();
        for pkg in &aur_packages {
            if crate::pacman::is_installed(&pkg.name)? {
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

        // Use the function from aur_install module
        if let Err(e) = install_aur_packages(
            &to_install.iter().map(|pkg| pkg.name.clone()).collect::<Vec<_>>(),
            config,
            noconfirm,
        ).await {
            let _ = crate::history::log_action("install", &to_install.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), false);
            return Err(e);
        } else {
             let _ = crate::history::log_action("install", &to_install.iter().map(|p| p.name.clone()).collect::<Vec<_>>(), true);
        }
    }

    // Install Flatpak packages
    if !flatpak_packages.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} Flatpak packages...", flatpak_packages.len()).bold());
        for app_id in flatpak_packages {
            if let Err(e) = crate::flatpak::install_flatpak(&app_id).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", app_id, e)));
                let _ = crate::history::log_action("install", &[app_id.clone()], false);
            } else {
                let _ = crate::history::log_action("install", &[app_id.clone()], true);
            }
        }
    }

    // Install Snap packages
    if !snap_packages.is_empty() {
        println!("\n{} {}", "::".bright_blue().bold(), format!("Installing {} Snap packages...", snap_packages.len()).bold());
        for name in snap_packages {
            if let Err(e) = crate::snap::install_snap(&name).await {
                eprintln!("{}", ui::error(&format!("Failed to install {}: {}", name, e)));
                let _ = crate::history::log_action("install", &[name.clone()], false);
            } else {
                let _ = crate::history::log_action("install", &[name.clone()], true);
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
                            let _ = crate::history::log_action("install", &[pkg.name.clone()], true);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", ui::error(&format!("Failed to download {}: {}", pkg.name, e)));
                        let _ = crate::history::log_action("install", &[pkg.name.clone()], false);
                    }
                }
            }
        } else {
            eprintln!("{}", ui::warning("Skipping Debian packages: debtap not available"));
        }
    }

    Ok(())
}