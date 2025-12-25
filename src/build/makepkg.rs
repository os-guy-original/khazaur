use crate::aur::AurPackage;
use crate::config::Config;
use crate::error::{KhazaurError, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;
use tracing::info;

/// Check if the output from makepkg contains PGP-related errors






/// Build and install a package using makepkg
pub fn build_and_install(package_dir: &Path, install: bool) -> Result<()> {
    info!("Building package in {:?}", package_dir);

    // Check if PKGBUILD exists
    let pkgbuild = package_dir.join("PKGBUILD");
    if !pkgbuild.exists() {
        return Err(KhazaurError::BuildFailed(
            "PKGBUILD not found".to_string(),
        ));
    }

    // Build arguments
    let mut args = vec!["-s"]; // Install dependencies
    if install {
        args.push("-i"); // Install after building
    }

    // First, try running makepkg with user interaction allowed (don't capture output)
    // But we'll try to run with better error handling
    let status = Command::new("makepkg")
        .args(&args)
        .current_dir(package_dir)
        .status()?;

    if !status.success() {
        let exit_code = status.code().unwrap_or(-1);

        // Exit code 8 typically means dependency resolution failed
        if exit_code == 8 {
            // Let's try to determine what dependencies are needed and install them separately
            let srcinfo_output = Command::new("makepkg")
                .arg("--printsrcinfo")
                .current_dir(package_dir)
                .output()?;

            if srcinfo_output.status.success() {
                let srcinfo = String::from_utf8_lossy(&srcinfo_output.stdout);
                let deps: Vec<String> = extract_dependencies_from_srcinfo(&srcinfo);

                if !deps.is_empty() {
                    eprintln!("Dependencies needed: {:?}", deps);
                    eprintln!("Attempting to install dependencies manually...");

                    // Try to install dependencies using pacman first
                    let mut deps_installed = true;
                    for dep in &deps {
                        // Check if dependency is already installed
                        if !is_package_installed(dep)? {
                            eprintln!("Installing dependency: {}", dep);
                            match crate::pacman::install_packages(&[dep.clone()], &vec!["--noconfirm".to_string()]) {
                                Ok(()) => {
                                    info!("Dependency {} installed successfully", dep);
                                }
                                Err(e) => {
                                    eprintln!("Failed to install dependency {}: {}", dep, e);
                                    deps_installed = false;
                                    break;
                                }
                            }
                        } else {
                            info!("Dependency {} already installed", dep);
                        }
                    }

                    if deps_installed {
                        // Try building again after installing dependencies
                        eprintln!("Dependencies installed, retrying build...");
                        let retry_status = Command::new("makepkg")
                            .args(&args)
                            .current_dir(package_dir)
                            .status()?;

                        if !retry_status.success() {
                            return Err(KhazaurError::BuildFailed(
                                format!("makepkg failed after installing dependencies: {}", retry_status),
                            ));
                        }

                        info!("Package built successfully after dependency installation");
                        return Ok(());
                    }
                }
            }

            return Err(KhazaurError::BuildFailed(
                "\nDependency installation failed.\n\n\
                 This can happen if you:\n\
                 • Interrupted the operation (Ctrl+C)\n\
                 • Rejected removing a conflicting package\n\
                 • Have network/download issues\n\n\
                 Try: khazaur -S <deps> to install dependencies manually".to_string()
            ));
        }

        return Err(KhazaurError::BuildFailed(
            format!("makepkg failed with status: {}", status),
        ));
    }

    info!("Package built successfully");
    Ok(())
}

/// Extract dependencies from srcinfo output
fn extract_dependencies_from_srcinfo(srcinfo: &str) -> Vec<String> {
    let mut deps = Vec::new();

    // Parse the srcinfo to extract dependencies
    for line in srcinfo.lines() {
        let line = line.trim();

        // Look for depends lines
        if line.starts_with("depends = ") {
            let dep = line.trim_start_matches("depends = ").trim();
            if !dep.is_empty() && !deps.contains(&dep.to_string()) {
                deps.push(dep.to_string());
            }
        }
        // Look for makedepends lines
        else if line.starts_with("makedepends = ") {
            let dep = line.trim_start_matches("makedepends = ").trim();
            if !dep.is_empty() && !deps.contains(&dep.to_string()) {
                deps.push(dep.to_string());
            }
        }
        // Look for checkdepends lines
        else if line.starts_with("checkdepends = ") {
            let dep = line.trim_start_matches("checkdepends = ").trim();
            if !dep.is_empty() && !deps.contains(&dep.to_string()) {
                deps.push(dep.to_string());
            }
        }
    }

    deps
}

/// Check if a package is installed using pacman
fn is_package_installed(pkg_name: &str) -> Result<bool> {
    let output = Command::new("pacman")
        .args(["-Q", pkg_name])
        .output()?;

    Ok(output.status.success())
}

/// Build and install a package using makepkg, with optional make dependency removal
pub fn build_and_install_with_make_deps_cleanup(
    package_dir: &Path,
    install: bool,
    pkg: &AurPackage,
    config: &Config,
    remove_make_deps: bool,
) -> Result<()> {
    // First, build and install the package normally
    build_and_install(package_dir, install)?;

    // If requested, remove make dependencies after successful installation
    if remove_make_deps {
        remove_make_dependencies(pkg, config)?;
    }

    Ok(())
}

/// Remove make dependencies that were installed for building the package
fn remove_make_dependencies(pkg: &AurPackage, _config: &Config) -> Result<()> {
    if pkg.make_depends.is_empty() {
        return Ok(());
    }

    println!("\n{} {}", "::".bright_blue().bold(), format!("Removing make dependencies for {}...", pkg.name).bold());

    // Filter make dependencies to only include packages that are currently installed
    let installed_make_deps: Vec<String> = pkg.make_depends
        .iter()
        .filter(|dep| {
            // Check if the package is installed using pacman
            crate::pacman::is_installed(dep).unwrap_or(false)
        })
        .cloned()
        .collect();

    if installed_make_deps.is_empty() {
        println!("{} {}", "::".yellow().bold(), "No installed make dependencies to remove".bold());
        return Ok(());
    }

    // Check if these dependencies are required by other packages
    let mut deps_to_remove = Vec::new();
    for dep in &installed_make_deps {
        // Check if the package is only installed as a dependency and not explicitly installed
        let output = std::process::Command::new("pacman")
            .args(["-Q", dep])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                // Check if the package was explicitly installed or just as a dependency
                // We'll use pacman -Qi to get more detailed info
                let info_output = std::process::Command::new("pacman")
                    .args(["-Qi", dep])
                    .output();

                if let Ok(info_output) = info_output {
                    if info_output.status.success() {
                        let info = String::from_utf8_lossy(&info_output.stdout);
                        // Look for "Install Reason:" field
                        if let Some(install_reason_line) = info.lines()
                            .find(|line| line.trim().starts_with("Install Reason:"))
                        {
                            if install_reason_line.contains("Dependency") {
                                // This package was installed as a dependency, safe to remove
                                deps_to_remove.push(dep.clone());
                            } else {
                                // This package was explicitly installed, don't remove
                                println!("{} {} was explicitly installed, keeping it",
                                    "::".yellow().bold(), dep);
                            }
                        } else {
                            // If we can't determine the install reason, assume it's safe to remove
                            deps_to_remove.push(dep.clone());
                        }
                    }
                }
            }
        }
    }

    if !deps_to_remove.is_empty() {
        println!("{} Removing: {}", "::".bright_blue().bold(), deps_to_remove.join(", "));

        // Remove the packages that were installed as dependencies
        match crate::pacman::remove_packages(&deps_to_remove, &["--noconfirm".to_string(), "--recursive".to_string()]) {
            Ok(()) => {
                println!("{} Make dependencies removed successfully", crate::ui::success("✓"));
            }
            Err(e) => {
                eprintln!("{} Failed to remove make dependencies: {}", crate::ui::error("✗"), e);
            }
        }
    } else {
        println!("{} No make dependencies to remove", crate::ui::info("ℹ"));
    }

    Ok(())
}
