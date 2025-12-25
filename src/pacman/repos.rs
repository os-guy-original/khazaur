use crate::error::{KhazaurError, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;

pub struct PacmanRepo {
    pub name: String,
    pub url: String,
}

const PACMAN_CONF: &str = "/etc/pacman.conf";

/// List repositories found in /etc/pacman.conf
/// This is a simple parser that looks for [section] followed by Server = ...
pub fn list_repos() -> Result<Vec<PacmanRepo>> {
    let file = File::open(PACMAN_CONF).map_err(|e| KhazaurError::Config(format!("Failed to open {}: {}", PACMAN_CONF, e)))?;
    let reader = BufReader::new(file);

    let mut repos = Vec::new();
    let mut current_section = String::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed[1..trimmed.len()-1].to_string();
            // Skip options section
            if current_section == "options" {
                current_section.clear();
            }
        } else if trimmed.starts_with("Server") && !current_section.is_empty() {
            if let Some(url_part) = trimmed.split('=').nth(1) {
                let url = url_part.trim().to_string();
                repos.push(PacmanRepo {
                    name: current_section.clone(),
                    url,
                });
                // Once we found a server for this section, we record it. 
                // Note: mirrors can have multiple servers or Include = ... which we might miss here for standard repos.
                // This is mostly for custom repos added by user which usually look like:
                // [repo]
                // Server = url
                
                // Clear section to avoid duplicates if multiple server lines (though usually handled by mirrorlist)
                current_section.clear();
            }
        } else if trimmed.starts_with("Include") && !current_section.is_empty() {
             if let Some(path) = trimmed.split('=').nth(1) {
                 repos.push(PacmanRepo {
                     name: current_section.clone(),
                     url: format!("Include = {}", path.trim()),
                 });
                 current_section.clear();
             }
        }
    }

    Ok(repos)
}

pub fn add_repo(name: &str, url: &str, siglevel: Option<&str>) -> Result<()> {
    // We need sudo to write to /etc/pacman.conf
    // Use a temporary file approach or echo append?
    // Echo append is simplest but need to use sh -c
    
    let mut content = format!("\n[{}]\nServer = {}\n", name, url);
    if let Some(sig) = siglevel {
        content = format!("\n[{}]\nSigLevel = {}\nServer = {}\n", name, sig, url);
    }

    let status = Command::new("sudo")
        .args(["sh", "-c", &format!("echo '{}' >> {}", content, PACMAN_CONF)])
        .status()?;

    if !status.success() {
        return Err(KhazaurError::Config("Failed to append to pacman.conf".to_string()));
    }

    Ok(())
}

pub fn remove_repo(name: &str) -> Result<()> {
    // Removing is tricky with sed safely.
    // We want to comment out:
    // [name]
    // Server = ...
    // SigLevel = ... (optional)
    
    // We will use sed to comment out the block [name] until the next [section] or end of file.
    // sed -i '/^\[name\]/,/^\[/ s/^/#/' /etc/pacman.conf
    // But this might comment out the next section header too.
    
    // Better approach:
    // sed -i '/^\[name\]/,/^\[/ { /^\[name\]/ s/^/#/; /Server/ s/^/#/; /SigLevel/ s/^/#/; }' 
    // This is getting complex and risky for automation without verifying.
    
    // Let's rely on a simpler sed pattern:
    // 1. Comment out the section header [name] -> #[name]
    // 2. We can't easily auto-comment the properties without knowing they belong to that section.
    
    // Safe generic implementation:
    // Read file, process in memory (Rust), write back as root (via dd or cp).
    // This allows robust logic.
    
    let file = File::open(PACMAN_CONF).map_err(|e| KhazaurError::Config(format!("Failed to open {}", e)))?;
    let reader = BufReader::new(file);
    
    let mut new_lines = Vec::new();
    let mut in_target_section = false;
    
    let target_header = format!("[{}]", name);
    
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        
        if trimmed == target_header {
            in_target_section = true;
            new_lines.push(format!("#{} (Disabled by Khazaur)", line));
            continue;
        }
        
        if in_target_section && trimmed.starts_with('[') {
            in_target_section = false;
        }
        
        if in_target_section {
            new_lines.push(format!("#{}", line));
        } else {
            new_lines.push(line);
        }
    }
    
    // Write new content to a temp file
    let temp_path = "/tmp/khazaur_pacman_conf_tmp";
    let mut temp_file = File::create(temp_path).map_err(|e| KhazaurError::Config(format!("Failed to create temp file: {}", e)))?;
    for line in new_lines {
        writeln!(temp_file, "{}", line)?;
    }
    
    // Move temp file to /etc/pacman.conf with sudo
    let status = Command::new("sudo")
        .args(["mv", temp_path, PACMAN_CONF])
        .status()?;
        
    if !status.success() {
        return Err(KhazaurError::Config("Failed to update pacman.conf".to_string()));
    }
    
    Ok(())
}

pub struct SuggestedRepo {
    pub name: String,
    pub server: String,
    pub description: String,
}

pub async fn fetch_suggested_repos() -> Result<Vec<SuggestedRepo>> {
    use regex::Regex;
    
    let url = "https://wiki.archlinux.org/title/Unofficial_user_repositories";
    
    let response = reqwest::get(url).await
        .map_err(|e| KhazaurError::Config(format!("Failed to fetch repos list: {}", e)))?
        .text().await
        .map_err(|e| KhazaurError::Config(format!("Failed to read response: {}", e)))?;

    let mut suggestions = Vec::new();
    
    // Regex for repo name in brackets: [reponame]
    let name_regex = Regex::new(r"^\[([a-zA-Z0-9_-]+)\]$").unwrap();
    // Regex for Server = URL line
    let server_regex = Regex::new(r"^Server\s*=\s*(\S+)").unwrap();
    // HTML header regex for descriptions
    let header_regex = Regex::new(r#"<h3.*?id="([^"]+)".*?>"#).unwrap();
    
    let mut current_section = String::new();
    let mut current_repo_name: Option<String> = None;
    
    for line in response.lines() {
        let trimmed = line.trim();
        
        // Strip HTML tags for cleaner matching
        let clean_line = trimmed
            .replace("<pre>", "")
            .replace("</pre>", "")
            .trim()
            .to_string();
        
        // Track section headers (for descriptions)
        if let Some(caps) = header_regex.captures(trimmed) {
            if let Some(header) = caps.get(1) {
                current_section = header.as_str().replace('_', " ");
            }
        }
        
        // Check for repo name [name]
        if let Some(caps) = name_regex.captures(&clean_line) {
            if let Some(name) = caps.get(1) {
                let name_str = name.as_str().to_string();
                // Skip standard repos
                if !["core", "extra", "multilib", "testing", "community", "options"].contains(&name_str.as_str()) {
                    current_repo_name = Some(name_str);
                }
            }
        }
        
        // Check for Server = URL line
        if let Some(caps) = server_regex.captures(&clean_line) {
            if let (Some(repo_name), Some(server)) = (&current_repo_name, caps.get(1)) {
                let server_str = server.as_str().to_string();
                
                // Avoid duplicates
                if !suggestions.iter().any(|r: &SuggestedRepo| r.name == *repo_name) {
                    suggestions.push(SuggestedRepo {
                        name: repo_name.clone(),
                        server: server_str,
                        description: current_section.clone(),
                    });
                }
                current_repo_name = None; // Reset after capturing
            }
        }
    }
    
    Ok(suggestions)
}
