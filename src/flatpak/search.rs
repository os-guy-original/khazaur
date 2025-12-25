use crate::error::Result;
use std::process::{Command, Stdio};
use super::types::FlatpakPackage;

/// Search for flatpak packages
pub fn search_flatpak(query: &str, no_timeout: bool) -> Result<Vec<FlatpakPackage>> {
    if !super::is_available() {
        return Ok(Vec::new());
    }

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
    // REMOVED: Fetching all packages via remote-ls is too slow and causes the "takes ages" issue.
    // relying on flatpak search (local appstream) is standard.
    
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
