use crate::error::{KhazaurError, Result};
use std::process::Command;

pub struct FlatpakRemote {
    pub name: String,
    pub title: String,
    pub url: String,
}

pub fn list_remotes() -> Result<Vec<FlatpakRemote>> {
    if !super::is_available() {
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(["remotes", "--columns=name,title,url"])
        .output()?;

    if !output.status.success() {
        return Err(KhazaurError::Config("Failed to list flatpak remotes".to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut remotes = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() || line.starts_with("Name") {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            remotes.push(FlatpakRemote {
                name: parts[0].trim().to_string(),
                title: parts[1].trim().to_string(),
                url: parts[2].trim().to_string(),
            });
        }
    }

    Ok(remotes)
}

pub fn add_remote(name: &str, url: &str) -> Result<()> {
    if !super::is_available() {
        return Err(KhazaurError::Config("Flatpak is not installed".to_string()));
    }

    // args: remote-add --if-not-exists <name> <url>
    let status = Command::new("sudo")
        .args(["flatpak", "remote-add", "--if-not-exists", name, url])
        .status()?;

    if !status.success() {
        return Err(KhazaurError::Config(format!("Failed to add remote: {}", name)));
    }

    Ok(())
}

pub fn remove_remote(name: &str) -> Result<()> {
    if !super::is_available() {
        return Err(KhazaurError::Config("Flatpak is not installed".to_string()));
    }

    let status = Command::new("sudo")
        .args(["flatpak", "remote-delete", "--force", name])
        .status()?;

    if !status.success() {
        return Err(KhazaurError::Config(format!("Failed to remove remote: {}", name)));
    }

    Ok(())
}

pub struct SuggestedRemote {
    pub name: String,
    pub title: String,
    pub url: String,
    pub description: String,
}

pub async fn fetch_suggested_remotes() -> Result<Vec<SuggestedRemote>> {
    use regex::Regex;
    
    // URL containing the list of remotes
    let url = "https://raw.githubusercontent.com/os-guy-original/flatpak-remotes/main/README.md";
    
    let response = reqwest::get(url).await
        .map_err(|e| KhazaurError::Config(format!("Failed to fetch remotes list: {}", e)))?
        .text().await
        .map_err(|e| KhazaurError::Config(format!("Failed to read response body: {}", e)))?;

    let mut suggestions = Vec::new();
    
    // Regex to match: flatpak remote-add --if-not-exists <name> <url>
    // It handles optional flags like --subset=verified
    let cmd_regex = Regex::new(r"flatpak remote-add.*?\s+([-\w]+)\s+(https?://\S+|oci\+https?://\S+)").unwrap();
    
    // Simple state machine to associate descriptions (headers) with commands
    let mut current_section = "Unknown".to_string();
    
    for line in response.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with('#') {
            current_section = trimmed.trim_start_matches('#').trim().to_string();
            // Clean up link markdown if present [Title](url) -> Title
            if current_section.starts_with('[') && current_section.contains("](") {
                if let Some(end_bracket) = current_section.find(']') {
                    current_section = current_section[1..end_bracket].to_string();
                }
            }
        } else if let Some(caps) = cmd_regex.captures(trimmed) {
            if let (Some(name), Some(url)) = (caps.get(1), caps.get(2)) {
                let name_str = name.as_str().to_string();
                
                // Avoid duplicates
                if !suggestions.iter().any(|r: &SuggestedRemote| r.name == name_str) {
                    suggestions.push(SuggestedRemote {
                        name: name_str,
                        title: current_section.clone(),
                        url: url.as_str().to_string(),
                        description: format!("Source: {}", current_section),
                    });
                }
            }
        }
    }
    
    Ok(suggestions)
}
