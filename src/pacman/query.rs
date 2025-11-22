use crate::error::Result;
use std::process::Command;

/// Check if a package is installed
pub fn is_installed(package_name: &str) -> Result<bool> {
    let output = Command::new("pacman")
        .args(["-Q", package_name])
        .output()?;
    
    Ok(output.status.success())
}



/// Search for packages in official repositories
pub fn search_repos(query: &str) -> Result<Vec<RepoPackage>> {
    let output = Command::new("pacman")
        .args(["-Ss", query])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages = parse_search_output(&stdout);
    
    Ok(packages)
}

/// Get information about a package from repositories
pub fn get_repo_info(package_name: &str) -> Result<Option<String>> {
    let output = Command::new("pacman")
        .args(["-Si", package_name])
        .output()?;
    
    if !output.status.success() {
        return Ok(None);
    }
    
    Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
}

/// Simple package information from repo search
#[derive(Debug, Clone)]
pub struct RepoPackage {
    pub repository: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

/// Parse pacman -Ss output
fn parse_search_output(output: &str) -> Vec<RepoPackage> {
    let mut packages = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        
        // First line: repo/name version [installed]
        if let Some((repo_name, rest)) = line.split_once(' ') {
            if let Some((repo, name)) = repo_name.split_once('/') {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                let version = parts.get(0).unwrap_or(&"").to_string();
                let installed = line.contains("[installed]");
                
                // Second line: description
                let description = if i + 1 < lines.len() {
                    lines[i + 1].trim().to_string()
                } else {
                    String::new()
                };
                
                packages.push(RepoPackage {
                    repository: repo.to_string(),
                    name: name.to_string(),
                    version,
                    description,
                    installed,
                });
                
                i += 2; // Skip description line
                continue;
            }
        }
        
        i += 1;
    }
    
    packages
}

/// Get detailed package information from repositories
pub fn get_package_details(package_name: &str) -> Result<Option<RepoPackage>> {
    let output = Command::new("pacman")
        .args(["-Si", package_name])
        .output()?;
    
    if !output.status.success() {
        return Ok(None);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut repo = String::new();
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            
            match key {
                "Repository" => repo = value.to_string(),
                "Name" => name = value.to_string(),
                "Version" => version = value.to_string(),
                "Description" => description = value.to_string(),
                _ => {}
            }
        }
    }
    
    if name.is_empty() {
        return Ok(None);
    }
    
    // Check if installed
    let installed = is_installed(&name)?;
    
    Ok(Some(RepoPackage {
        repository: repo,
        name,
        version,
        description,
        installed,
    }))
}


/// Search for installed packages matching a query (fuzzy search)
pub fn search_installed_packages(query: &str) -> Result<Vec<String>> {
    let output = Command::new("pacman")
        .args(["-Qq"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let query_lower = query.to_lowercase();
    
    let matches: Vec<String> = stdout
        .lines()
        .filter(|line| line.to_lowercase().contains(&query_lower))
        .map(|s| s.to_string())
        .collect();
    
    Ok(matches)
}

/// Get all installed packages with their versions
pub fn get_installed_packages() -> Result<Vec<(String, String)>> {
    let output = Command::new("pacman")
        .args(["-Q"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<(String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();
    
    Ok(packages)
}

/// Get all installed AUR packages (packages not in official repos)
pub fn get_installed_aur_packages() -> Result<Vec<(String, String)>> {
    let output = Command::new("pacman")
        .args(["-Qm"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let packages: Vec<(String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();
    
    Ok(packages)
}

/// Get available repository package updates
pub fn get_repo_updates() -> Result<Vec<(String, String, String)>> {
    // Run pacman -Qu to get available updates
    let output = Command::new("pacman")
        .args(["-Qu"])
        .output()?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let updates: Vec<(String, String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                // Format: package old_version -> new_version
                Some((parts[0].to_string(), parts[1].to_string(), parts[3].to_string()))
            } else {
                None
            }
        })
        .collect();
    
    Ok(updates)
}
