use crate::aur::AurClient;
use crate::config::Config;
use crate::error::Result;
use crate::ui;
use tracing::info;

/// Search for packages in AUR and/or repos
pub async fn search(
    query: &str,
    config: &mut Config,
    aur_only: bool,
    repo_only: bool,
    only_aur: bool,
    only_repos: bool,
    only_flatpak: bool,
    only_snap: bool,
    only_debian: bool,
) -> Result<()> {
    println!("{}", ui::section_header(&format!("Searching for '{}'", query)));

    let client = AurClient::new()?;
    
    // Combine old and new flags
    let aur_filter = aur_only || only_aur;
    let repo_filter = repo_only || only_repos;
    let flatpak_filter = only_flatpak;
    let snap_filter = only_snap;
    let debian_filter = only_debian;
    
    // If no specific source requested, search all
    let search_all = !aur_filter && !repo_filter && !flatpak_filter && !snap_filter && !debian_filter;
    
    // Prompt for optional dependencies BEFORE searching if needed
    if search_all || flatpak_filter {
        if !crate::flatpak::is_available() {
            crate::cli::optional_deps::check_and_prompt_flatpak(config).await?;
        }
    }
    
    if search_all || snap_filter {
        if !crate::snap::is_available() {
            crate::cli::optional_deps::check_and_prompt_snapd(config).await?;
        }
    }
    
    if search_all || debian_filter {
        if !crate::debtap::is_available() {
            crate::cli::optional_deps::check_and_prompt_debtap(config).await?;
        }
    }

    // Search repositories
    if search_all || repo_filter {
        info!("Searching repositories...");
        let repo_packages = crate::pacman::search_repos(query)?;
        
        if !repo_packages.is_empty() {
            println!("\n{}", ui::info(&format!("Repository Packages ({})", repo_packages.len())));
            println!("{}", ui::format_repo_packages(&repo_packages));
        }
    }

    // Search AUR
    if search_all || aur_filter {
        info!("Searching AUR...");
        let spinner = ui::spinner("Searching AUR...");
        let aur_result = client.search(query).await;
        spinner.finish_and_clear();
        
        match aur_result {
            Ok(aur_packages) => {
                if !aur_packages.is_empty() {
                    println!("\n{}", ui::info(&format!("AUR Packages ({})", aur_packages.len())));
                    // Pass false to skip slow is_installed checks during search
                    println!("{}", ui::format_aur_packages(&aur_packages, false));
                } else {
                    println!("{}", ui::warning("No AUR packages found"));
                }
            }
            Err(e) => {
                // Check for "Too many results" error
                let error_msg = e.to_string();
                if error_msg.contains("Too many package results") {
                    println!("\n{}", ui::warning("Search query too broad"));
                    println!("{}", ui::info("Tip: Be more specific with your search query to get results"));
                    println!("     Example: Instead of 'rust', try 'rust-analyzer'");
                } else {
                    // Other errors
                    return Err(e);
                }
            }
        }
    }


    // Search Flatpak (only if available)
    if (search_all || flatpak_filter) && crate::flatpak::is_available() {
        info!("Searching Flatpak...");
        match crate::flatpak::search_flatpak(query, false) {
            Ok(flatpak_packages) if !flatpak_packages.is_empty() => {
                println!("\n{}", ui::info(&format!("Flatpak Apps ({})", flatpak_packages.len())));
                println!("{}", ui::format_flatpak_packages(&flatpak_packages));
            }
            Ok(_) => {
                info!("No flatpak apps found");
            }
            Err(e) => {
                info!("Flatpak search error: {}", e);
            }
        }
    }

    // Search Snap (only if available) - run in background to not block
    if (search_all || snap_filter) && crate::snap::is_available() {
        info!("Searching Snap...");
        let query_clone = query.to_string();
        let snap_handle = tokio::task::spawn_blocking(move || {
            crate::snap::search_snap(&query_clone)
        });

        match snap_handle.await {
            Ok(Ok(snap_packages)) if !snap_packages.is_empty() => {
                println!("\n{}", ui::info(&format!("Snap Packages ({})", snap_packages.len())));
                println!("{}", ui::format_snap_packages(&snap_packages));
            }
            Ok(Ok(_)) => {
                info!("No snap packages found");
            }
            Ok(Err(e)) => {
                info!("Snap search error: {}", e);
            }
            Err(e) => {
                tracing::error!("Error searching snaps: {}", e);
            }
        }
    }
    
    // Search Debian (only if debtap is available)
    if (search_all || debian_filter) && crate::debtap::is_available() {
        info!("Searching Debian...");
        let spinner = ui::spinner("Searching Debian...");
        match crate::debian::search_debian(query).await {
            Ok(packages) => {
                spinner.finish_and_clear();
                if !packages.is_empty() {
                    println!("\n{}", ui::info(&format!("Debian Packages ({})", packages.len())));
                    println!("{}", ui::format_debian_packages(&packages));
                }
            }
            Err(e) => {
                spinner.finish_and_clear();
                tracing::error!("Error searching Debian: {}", e);
            }
        }
    }
    
    Ok(())
}

/// Show detailed package information
pub async fn show_info(package_name: &str, _config: &Config) -> Result<()> {
    let client = AurClient::new()?;

    // Try AUR first
    match client.info(package_name).await {
        Ok(pkg) => {
            println!("{}", ui::format_aur_info(&pkg));
            return Ok(());
        }
        Err(_) => {
            // Try repository
            if let Some(info) = crate::pacman::get_repo_info(package_name)? {
                println!("{}", ui::section_header(&format!("Repository Package: {}", package_name)));
                println!("{}", info);
                return Ok(());
            }
        }
    }

    println!("{}", ui::error(&format!("Package '{}' not found", package_name)));
    Ok(())
}
