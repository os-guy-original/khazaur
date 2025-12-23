use crate::error::{KhazaurError, Result};
use std::process::Command;
use serde::{Deserialize, Serialize};

pub mod updates;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatpakPackage {
    pub name: String,
    pub app_id: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub description: String,
}



/// Check if flatpak is installed
pub fn is_available() -> bool {
    Command::new("which")
        .arg("flatpak")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Search for flatpak packages
pub fn search_flatpak(query: &str, no_timeout: bool) -> Result<Vec<FlatpakPackage>> {
    if !is_available() {
        return Ok(Vec::new());
    }

    // Set a timeout to prevent hanging (unless disabled)
    use std::process::Stdio;

    // Try the search with the original query first
    let output = if no_timeout {
        // No timeout - run flatpak search directly
        Command::new("flatpak")
            .args(["search", "--columns=name,description,application,version,branch", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?
    } else {
        // Use timeout command to prevent hanging
        let timeout_result = Command::new("timeout")
            .args(["5", "flatpak", "search", "--columns=name,description,application,version,branch", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match timeout_result {
            Ok(o) => o,
            Err(_) => {
                // Try without timeout command (might not be available)
                let o = Command::new("flatpak")
                    .args(["search", "--columns=name,description,application,version,branch", query])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?;
                o
            }
        }
    };

    let mut packages = if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_flatpak_search(&stdout)
    } else {
        Vec::new()
    };

    // If the initial search didn't find good matches, try some transformations for a-b format packages
    if packages.is_empty() || query.contains('-') {
        // Transform a-b format to common Flatpak naming conventions
        let transformed_query = transform_query_for_flatpak(query);

        if transformed_query != query {
            let output = if no_timeout {
                Command::new("flatpak")
                    .args(["search", "--columns=name,description,application,version,branch", &transformed_query])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?
            } else {
                let timeout_result = Command::new("timeout")
                    .args(["5", "flatpak", "search", "--columns=name,description,application,version,branch", &transformed_query])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output();

                let o = match timeout_result {
                    Ok(o) => o,
                    Err(_) => {
                        let o = Command::new("flatpak")
                            .args(["search", "--columns=name,description,application,version,branch", &transformed_query])
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null())
                            .output()?;
                        o
                    }
                };
                o
            };

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                packages.extend(parse_flatpak_search(&stdout));
            }
        }
    }

    // If still no results, try to get all available packages and do fuzzy matching
    if packages.is_empty() {
        let all_packages = get_all_flatpak_packages(no_timeout)?;
        packages = fuzzy_match_packages(&all_packages, query);
    }

    Ok(packages)
}

/// Transform a query to potential Flatpak naming conventions based on patterns
fn transform_query_for_flatpak(query: &str) -> String {
    // Handle various separators: convert dots, underscores, and hyphens to a common format
    let mut possible_transformations = Vec::new();

    // If query contains dots, try treating first part as domain
    if query.contains('.') {
        let parts: Vec<&str> = query.split('.').collect();
        if parts.len() >= 2 {
            let domain = parts[0];
            let app_name = parts[1..].iter()
                .map(|&s| capitalize_first(s))
                .collect::<Vec<_>>()
                .join(".");
            possible_transformations.push(format!("{}.{}", domain, app_name));
        }
    }

    // If query contains hyphens, try various transformations
    if query.contains('-') {
        let parts: Vec<&str> = query.split('-').collect();
        if parts.len() >= 2 {
            // Try common domain patterns
            let common_domains = ["org", "com", "io", "net", "app"];
            for domain in common_domains {
                if parts[0].to_lowercase() == domain && parts.len() > 1 {
                    // If the first part is a common domain, use it as the domain
                    let app_name = parts[1..].iter()
                        .map(|&s| capitalize_first(s))
                        .collect::<Vec<_>>()
                        .join(".");
                    possible_transformations.push(format!("{}.{}", domain, app_name));
                }
            }

            // Default to org prefix and capitalize the rest
            if possible_transformations.is_empty() {
                let app_name = parts[1..].iter()
                    .map(|&s| capitalize_first(s))
                    .collect::<Vec<_>>()
                    .join(".");
                possible_transformations.push(format!("org.{}.{}", parts[0], app_name));
            }
        }
    }

    // If query contains underscores
    if query.contains('_') {
        let parts: Vec<&str> = query.split('_').collect();
        if parts.len() >= 2 {
            let app_name = parts[1..].iter()
                .map(|&s| capitalize_first(s))
                .collect::<Vec<_>>()
                .join(".");
            possible_transformations.push(format!("org.{}.{}", parts[0], app_name));
        }
    }

    // If we have transformations, return the first one
    if let Some(transformation) = possible_transformations.first() {
        return transformation.clone();
    }

    // If no special transformation applies, return the original query
    query.to_string()
}

/// Capitalize the first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Get all available Flatpak packages (for fuzzy matching when search fails)
fn get_all_flatpak_packages(no_timeout: bool) -> Result<Vec<FlatpakPackage>> {
    use std::process::Stdio;

    let output = if no_timeout {
        Command::new("flatpak")
            .args(["remote-ls", "--columns=name,application,version,branch,description", "flathub"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?
    } else {
        let timeout_result = Command::new("timeout")
            .args(["10", "flatpak", "remote-ls", "--columns=name,application,version,branch,description", "flathub"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        let o = match timeout_result {
            Ok(o) => o,
            Err(_) => {
                let o = Command::new("flatpak")
                    .args(["remote-ls", "--columns=name,application,version,branch,description", "flathub"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()?;
                o
            }
        };
        o
    };

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_flatpak_remotes_list(&stdout))
}

/// Parse flatpak remote-ls output
fn parse_flatpak_remotes_list(output: &str) -> Vec<FlatpakPackage> {
    let mut packages = Vec::new();

    for line in output.lines() {
        // Skip empty lines and headers
        if line.trim().is_empty() || line.starts_with("Name") {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].trim();
            let app_id = parts.get(1).map_or(name, |v| v.trim());
            let version = parts.get(2).map_or("", |v| v.trim());
            let branch = parts.get(3).map_or("stable", |v| v.trim());
            let description = parts.get(4).map_or("", |v| v.trim());

            packages.push(FlatpakPackage {
                name: name.to_string(),
                app_id: app_id.to_string(),
                version: version.to_string(),
                branch: branch.to_string(),
                origin: "flathub".to_string(),
                description: description.to_string(),
            });
        }
    }

    packages
}

/// Perform fuzzy matching against a list of packages
fn fuzzy_match_packages(all_packages: &[FlatpakPackage], query: &str) -> Vec<FlatpakPackage> {
    let query_lower = query.to_lowercase();
    let mut scored_matches = Vec::new();

    for pkg in all_packages {
        // Check various fields for matches
        let name_lower = pkg.name.to_lowercase();
        let app_id_lower = pkg.app_id.to_lowercase();
        let description_lower = pkg.description.to_lowercase();

        // Calculate match score based on multiple factors
        let mut score = 0;

        // Exact name match gets high score
        if name_lower == query_lower {
            score += 100;
        }
        // Partial name match
        else if name_lower.contains(&query_lower) {
            score += 50;
        }
        // Query appears in app ID
        else if app_id_lower.contains(&query_lower) {
            score += 30;
        }
        // Query appears in description
        else if description_lower.contains(&query_lower) {
            score += 10;
        }

        // Check for various separator transformations
        // Convert query to different formats to match against app ID
        let query_as_hyphenated = query_lower.replace('.', "-").replace('_', "-");
        let query_as_dotted = query_lower.replace('-', ".").replace('_', ".");
        let query_as_underscored = query_lower.replace('-', "_").replace('.', "_");

        // Check if any transformed query matches the app ID
        if app_id_lower.replace('.', "-").replace('_', "-").contains(&query_as_hyphenated) {
            score += 25;
        }
        if app_id_lower.replace('-', ".").replace('_', ".").contains(&query_as_dotted) {
            score += 25;
        }
        if app_id_lower.replace('-', "_").replace('.', "_").contains(&query_as_underscored) {
            score += 25;
        }

        // Check if app ID parts match query parts (e.g., "google.chrome" matches "com.google.Chrome")
        let app_id_parts: Vec<&str> = app_id_lower.split('.').collect();
        let query_parts: Vec<&str> = query_lower.split(|c| c == '.' || c == '-' || c == '_').collect();

        // Count matching parts (at least 50% of query parts should match somewhere in app ID)
        let matching_parts = query_parts.iter()
            .filter(|&part| !part.is_empty() && app_id_lower.contains(part))
            .count();

        if query_parts.len() > 0 && matching_parts as f32 / query_parts.len() as f32 >= 0.5 {
            score += 20;
        }

        // Additional scoring for partial word matches in app ID
        // Split app ID by dots and see if query parts match any of them
        for part in &query_parts {
            if !part.is_empty() {
                for app_id_part in &app_id_parts {
                    if app_id_part.contains(part) || part.contains(app_id_part) {
                        score += 5; // Small bonus for partial matches
                    }
                }
            }
        }

        // NEW: Match against display name patterns
        // Convert app name to common search formats
        let display_name_variations = generate_name_variations(&name_lower);
        for variation in &display_name_variations {
            if variation.contains(&query_lower) || query_lower.contains(variation) {
                score += 15; // Bonus for matching display name variations
            }
        }

        // NEW: Check for matches between query and app name (e.g., "google chrome" vs "Google Chrome")
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        // Count how many query words appear in the name
        let matching_words = query_words.iter()
            .filter(|&word| !word.is_empty() && name_lower.contains(word))
            .count();

        if !query_words.is_empty() && matching_words as f32 / query_words.len() as f32 >= 0.5 {
            score += 35; // Bonus for matching significant portion of name words
        }

        // Additional bonus if all query words match
        if !query_words.is_empty() && matching_words == query_words.len() {
            score += 20; // Extra bonus for complete word matches
        }

        if score > 0 {
            scored_matches.push((pkg.clone(), score));
        }
    }

    // Sort by score (descending) to prioritize better matches
    scored_matches.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by score in descending order

    // Return top matches (limit to 10 to avoid too many results)
    scored_matches.into_iter().take(10).map(|(pkg, _)| pkg).collect()
}

/// Generate common variations of a display name for matching
/// e.g., "Google Chrome" -> ["google chrome", "google-chrome", "google.chrome", "google_chrome", "googlechrome"]
fn generate_name_variations(name: &str) -> Vec<String> {
    let mut variations = Vec::new();
    let lower_name = name.to_lowercase();

    // Add lowercase version
    variations.push(lower_name.clone());

    // Replace spaces with different separators
    variations.push(lower_name.replace(' ', "-"));
    variations.push(lower_name.replace(' ', "."));
    variations.push(lower_name.replace(' ', "_"));
    variations.push(lower_name.replace(' ', ""));

    // If name has multiple words, also add individual words
    let words: Vec<&str> = lower_name.split_whitespace().collect();
    for word in &words {
        if !word.is_empty() {
            variations.push(word.to_string());
        }
    }

    variations
}


/// Parse flatpak search output
fn parse_flatpak_search(output: &str) -> Vec<FlatpakPackage> {
    let mut packages = Vec::new();

    for line in output.lines() {
        // Skip empty lines and headers
        if line.trim().is_empty() || line.starts_with("Name") {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = parts[0].trim();
            let description = parts.get(1).map_or("", |v| v.trim());
            let app_id = parts.get(2).map_or(name, |v| v.trim());
            let version = parts.get(3).map_or("", |v| v.trim());
            let branch = parts.get(4).map_or("stable", |v| v.trim());
            // Origin not in output, default to flathub
            let origin = "flathub";

            packages.push(FlatpakPackage {
                name: name.to_string(),
                description: description.to_string(),
                app_id: app_id.to_string(),
                version: version.to_string(),
                branch: branch.to_string(),
                origin: origin.to_string(),
            });
        }
    }

    packages
}

/// Install a flatpak application
pub async fn install_flatpak(app_id: &str) -> Result<()> {
    use colored::Colorize;
    use tokio::process::Command;
    use tokio::signal;
    
    if !is_available() {
        return Err(KhazaurError::Config(
            "Flatpak is not installed on this system".to_string()
        ));
    }
    
    // Check if already installed
    if is_flatpak_installed(app_id)? {
        println!("{} {} {}", 
            "::".bright_blue().bold(),
            app_id.bold(),
            "is already installed".dimmed()
        );
        return Ok(())
    }
    
    println!("{} {}", "::".bright_blue().bold(), format!("Installing flatpak: {}", app_id).bold());
    
    let mut child = Command::new("flatpak")
        .args(["install", "-y", "flathub", app_id])
        .spawn()
        .map_err(|e| KhazaurError::Config(format!("Failed to start flatpak install: {}", e)))?;
        
    tokio::select! {
        status = child.wait() => {
            match status {
                Ok(s) if s.success() => {
                    println!("{}", format!("✓ {} installed successfully", app_id).green());
                    Ok(())
                }
                Ok(_) => {
                    Err(KhazaurError::Config(format!("Failed to install flatpak: {}", app_id)))
                }
                Err(e) => {
                    Err(KhazaurError::Config(format!("Failed to wait for flatpak process: {}", e)))
                }
            }
        }
        _ = signal::ctrl_c() => {
            println!("\n{}", ":: Installation cancelled by user".yellow());
            let _ = child.kill().await;
            Err(KhazaurError::Config("Installation cancelled".to_string()))
        }
    }
}

/// Check if a flatpak application is installed
pub fn is_flatpak_installed(app_id: &str) -> Result<bool> {
    if !is_available() {
        return Ok(false);
    }
    
    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()?;
    
    if !output.status.success() {
        return Ok(false);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().any(|line| line.trim() == app_id))
}

/// Get list of installed flatpak applications matching a query
pub fn get_installed_flatpaks(query: &str) -> Result<Vec<String>> {
    if !is_available() {
        return Ok(Vec::new());
    }
    
    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let query_lower = query.to_lowercase();
    
    let matches: Vec<String> = stdout
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|s| s.trim().to_string())
        .collect();
    
    Ok(matches)
}

/// Uninstall a flatpak application
pub fn uninstall_flatpak(app_id: &str) -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config(
            "Flatpak is not installed on this system".to_string()
        ));
    }
    
    let status = Command::new("flatpak")
        .args(["uninstall", "-y", app_id])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::Config(
            format!("Failed to uninstall flatpak: {}", app_id)
        ));
    }
    
    Ok(())
}


/// Get list of Flatpak packages with available updates
/// Returns Vec of (display_name, current_version, new_version)
pub fn get_updates() -> Result<Vec<(String, String, String)>> {
    updates::get_updates().map(|updates| {
        updates
            .into_iter()
            .map(|u| (format!("{} ({})", u.name, u.app_id), u.current_version, u.new_version))
            .collect()
    })
}


/// Update all Flatpak packages
pub fn update_all() -> Result<()> {
    if !is_available() {
        return Err(KhazaurError::Config("Flatpak is not installed".to_string()));
    }
    
    let status = Command::new("flatpak")
        .args(&["update", "-y"])
        .status()?;
    
    if !status.success() {
        return Err(KhazaurError::Config("Flatpak update failed".to_string()));
    }
    
    Ok(())
}