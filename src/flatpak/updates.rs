use crate::error::Result;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct FlatpakUpdate {
    pub name: String,
    pub app_id: String,
    pub current_version: String,
    pub new_version: String,
}

/// Get list of Flatpak packages with available updates
/// Uses `flatpak remote-ls --updates` to detect packages with updates,
/// and then fetches the remote version using `flatpak remote-info`
pub fn get_updates() -> Result<Vec<FlatpakUpdate>> {
    if !super::is_available() {
        return Ok(Vec::new());
    }

    // Get list of installed apps with their versions
    let installed_output = Command::new("flatpak")
        .args(["list", "--app", "--columns=name,application,version,origin"])
        .output()?;

    if !installed_output.status.success() {
        return Ok(Vec::new());
    }

    let installed_stdout = String::from_utf8_lossy(&installed_output.stdout);
    let installed_apps = parse_installed_apps(&installed_stdout);

    // Get list of apps with available updates
    let updates_output = Command::new("flatpak")
        .args(["remote-ls", "--updates", "--columns=application"])
        .output()?;

    if !updates_output.status.success() {
        return Ok(Vec::new());
    }

    let updates_stdout = String::from_utf8_lossy(&updates_output.stdout);
    let apps_with_updates: Vec<&str> = updates_stdout
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    let mut updates = Vec::new();

    for (name, app_id, current_version, origin) in &installed_apps {
        if apps_with_updates.iter().any(|id| id == app_id) {
            // Fetch the remote version
            let new_version = get_remote_version(app_id, origin)
                .unwrap_or_else(|_| current_version.clone());

            // Only report as update if versions actually differ
            if new_version != *current_version {
                updates.push(FlatpakUpdate {
                    name: name.clone(),
                    app_id: app_id.clone(),
                    current_version: current_version.clone(),
                    new_version,
                });
            }
        }
    }

    Ok(updates)
}

/// Parse the output of `flatpak list --app --columns=name,application,version,origin`
fn parse_installed_apps(output: &str) -> Vec<(String, String, String, String)> {
    let mut apps = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 4 {
            let name = parts[0].trim().to_string();
            let app_id = parts[1].trim().to_string();
            let version = parts[2].trim().to_string();
            let origin = parts[3].trim().to_string();
            apps.push((name, app_id, version, origin));
        } else if parts.len() >= 3 {
            // Fallback if origin is missing
            let name = parts[0].trim().to_string();
            let app_id = parts[1].trim().to_string();
            let version = parts[2].trim().to_string();
            apps.push((name, app_id, version, "flathub".to_string()));
        }
    }

    apps
}

/// Get the remote version of a Flatpak app from its origin
fn get_remote_version(app_id: &str, origin: &str) -> Result<String> {
    let output = Command::new("flatpak")
        .args(["remote-info", origin, app_id])
        .output()?;

    if !output.status.success() {
        // Try with flathub as fallback
        let output = Command::new("flatpak")
            .args(["remote-info", "flathub", app_id])
            .output()?;
        
        if !output.status.success() {
            return Err(crate::error::KhazaurError::Config(
                format!("Failed to get remote info for {}", app_id)
            ));
        }
        
        return parse_remote_version(&String::from_utf8_lossy(&output.stdout));
    }

    parse_remote_version(&String::from_utf8_lossy(&output.stdout))
}

/// Parse the version from `flatpak remote-info` output
fn parse_remote_version(output: &str) -> Result<String> {
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Version:") {
            return Ok(line
                .strip_prefix("Version:")
                .unwrap_or("")
                .trim()
                .to_string());
        }
    }

    Err(crate::error::KhazaurError::Config(
        "Could not parse version from remote info".to_string()
    ))
}
