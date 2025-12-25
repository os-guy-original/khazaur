use crate::aur::{download, AurClient, AurPackage};
use crate::build;
use crate::config::Config;
use crate::error::Result;
use crate::resolver::Resolver;
use crate::ui;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm};

/// Prompt user about removing make dependencies after installation
pub fn prompt_remove_make_deps(pkg: &AurPackage, noconfirm: bool) -> Result<bool> {
    if pkg.make_depends.is_empty() {
        return Ok(false);
    }

    if noconfirm {
        // If noconfirm is set, we assume the user wants the default behavior (don't remove)
        return Ok(false);
    }

    let make_deps_list = pkg.make_depends.join(", ");
    let prompt = format!(
        "Remove make dependencies ({}) after installing {}?",
        make_deps_list,
        pkg.name
    );

    let confirmed = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    Ok(confirmed)
}

/// Install AUR packages
pub async fn install_aur_packages(
    packages: &[String],
    config: &mut Config,
    noconfirm: bool,
) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let client = AurClient::new()?;
    
    // Filter out already installed packages
    let mut to_install = Vec::new();
    for pkg_name in packages {
        match client.info(pkg_name).await {
            Ok(pkg) => {
                if crate::pacman::is_installed(&pkg.name)? {
                    println!("{} {} {}",
                        "::".bright_blue().bold(),
                        pkg.name.bold(),
                        "is already installed".dimmed()
                    );
                } else {
                    to_install.push(pkg);
                }
            }
            Err(e) => {
                eprintln!("{}", ui::error(&format!("Failed to get info for {}: {}", pkg_name, e)));
            }
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

            // Prompt user about removing make dependencies after installation
            let remove_make_deps = prompt_remove_make_deps(pkg, noconfirm)?;

            println!("\n{} {}", "::".bright_cyan(), format!("Building {}...", pkg.name).bold());

            // Build and install with makepkg, with optional make dependency removal
            match build::build_and_install_with_make_deps_cleanup(pkg_dir, true, pkg, config, remove_make_deps) {
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

    Ok(())
}