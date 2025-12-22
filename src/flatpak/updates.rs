use crate::error::Result;
use regex::Regex;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct FlatpakUpdate {
    pub name: String,
    pub app_id: String,
    pub current_version: String,
    pub new_version: String,
}

/// Get list of Flatpak packages with available updates
/// Uses `flatpak remote-ls --updates` to detect packages with updates
/// Then uses `flatpak remote-info` to extract the actual new version from commit subject
pub fn get_updates() -> Result<Vec<FlatpakUpdate>> {
    if !super::is_available() {
        return Ok(Vec::new());
    }

    // Get list of installed apps with their versions and origin
    let installed_output = Command::new("flatpak")
        .args(["list", "--app", "--columns=name,application,version,origin"])
        .output()?;

    if !installed_output.status.success() {
        return Ok(Vec::new());
    }

    let installed_stdout = String::from_utf8_lossy(&installed_output.stdout);
    let installed_apps = parse_installed_apps(&installed_stdout);

    // Get list of app IDs with available updates
    let updates_output = Command::new("flatpak")
        .args(["remote-ls", "--updates", "--columns=application"])
        .output()?;

    if !updates_output.status.success() {
        return Ok(Vec::new());
    }

    let updates_stdout = String::from_utf8_lossy(&updates_output.stdout);
    let app_ids_with_updates: std::collections::HashSet<String> = updates_stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with("Application"))
        .collect();

    let mut updates = Vec::new();

    // For each installed app that has an update, get the real new version
    for (name, app_id, current_version, origin) in &installed_apps {
        if app_ids_with_updates.contains(app_id) {
            // Get the new version from remote-info (extracts from commit subject if possible)
            let new_version = get_real_remote_version(app_id, origin)
                .unwrap_or_else(|_| "update available".to_string());

            updates.push(FlatpakUpdate {
                name: name.clone(),
                app_id: app_id.clone(),
                current_version: current_version.clone(),
                new_version,
            });
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

/// Get the real remote version of a Flatpak app
/// First tries to extract version from commit subject (e.g., "update-to-1.17.15b")
/// Falls back to the Version field in remote-info
fn get_real_remote_version(app_id: &str, origin: &str) -> Result<String> {
    let output = Command::new("flatpak")
        .args(["remote-info", origin, app_id])
        .output()?;

    if !output.status.success() {
        // Try with flathub as fallback
        let output = Command::new("flatpak")
            .args(["remote-info", "flathub", app_id])
            .output()?;

        if !output.status.success() {
            return Err(crate::error::KhazaurError::Config(format!(
                "Failed to get remote info for {}",
                app_id
            )));
        }

        return parse_version_from_remote_info(&String::from_utf8_lossy(&output.stdout));
    }

    parse_version_from_remote_info(&String::from_utf8_lossy(&output.stdout))
}

/// Parse version from `flatpak remote-info` output
/// Tries to extract version from Subject line first (more accurate for recent updates)
/// Falls back to Version field
fn parse_version_from_remote_info(output: &str) -> Result<String> {
    let mut metadata_version = String::new();
    let mut subject_line = String::new();

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("Version:") {
            metadata_version = line
                .strip_prefix("Version:")
                .unwrap_or("")
                .trim()
                .to_string();
        } else if line.starts_with("Subject:") {
            subject_line = line
                .strip_prefix("Subject:")
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    // Try to extract version from subject line
    // Common patterns:
    // - "Merge pull request #XX from flathub/update-to-1.17.15b"
    // - "Update to 1.17.15b"
    // - "Bump version to 1.17.15b"
    // - "v1.17.15b"
    if !subject_line.is_empty() {
        if let Some(version) = extract_version_from_subject(&subject_line) {
            // Only use subject version if it's different from metadata version
            // This ensures we get the actual new version being deployed
            if version != metadata_version && !version.is_empty() {
                return Ok(version);
            }
        }
    }

    // Fall back to metadata version
    if !metadata_version.is_empty() {
        return Ok(metadata_version);
    }

    Err(crate::error::KhazaurError::Config(
        "Could not parse version from remote info".to_string(),
    ))
}

/// Extract version string from a commit subject line
/// Handles common patterns used in Flatpak commit messages
fn extract_version_from_subject(subject: &str) -> Option<String> {
    // Pattern 1: "update-to-VERSION" or "update-to VERSION" or "update to VERSION"
    let update_to_regex = Regex::new(r"(?i)update[-_\s]?to[-_\s]?v?([0-9][0-9a-zA-Z._-]*)").ok()?;
    if let Some(caps) = update_to_regex.captures(subject) {
        if let Some(version) = caps.get(1) {
            return Some(version.as_str().to_string());
        }
    }

    // Pattern 2: "bump version to VERSION" or "bump to VERSION"
    let bump_regex = Regex::new(r"(?i)bump\s+(?:version\s+)?to\s+v?([0-9][0-9a-zA-Z._-]*)").ok()?;
    if let Some(caps) = bump_regex.captures(subject) {
        if let Some(version) = caps.get(1) {
            return Some(version.as_str().to_string());
        }
    }

    // Pattern 3: Generic version pattern at word boundary (e.g., "Release 1.2.3" or "v1.2.3")
    // Only match if it looks like a deliberate version mention
    let version_regex = Regex::new(r"(?i)(?:release|version|^v)\s*v?([0-9]+\.[0-9][0-9a-zA-Z._-]*)").ok()?;
    if let Some(caps) = version_regex.captures(subject) {
        if let Some(version) = caps.get(1) {
            return Some(version.as_str().to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_subject() {
        // Test update-to patterns
        assert_eq!(
            extract_version_from_subject("Merge pull request #146 from flathub/update-to-1.17.15b"),
            Some("1.17.15b".to_string())
        );
        assert_eq!(
            extract_version_from_subject("update to 2.0.0"),
            Some("2.0.0".to_string())
        );
        assert_eq!(
            extract_version_from_subject("Update-to-v3.1.4"),
            Some("3.1.4".to_string())
        );

        // Test bump patterns
        assert_eq!(
            extract_version_from_subject("Bump version to 1.2.3"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            extract_version_from_subject("bump to 4.5.6"),
            Some("4.5.6".to_string())
        );

        // Test release patterns
        assert_eq!(
            extract_version_from_subject("Release 1.0.0"),
            Some("1.0.0".to_string())
        );
        assert_eq!(
            extract_version_from_subject("version 2.3.4"),
            Some("2.3.4".to_string())
        );

        // Test that random text doesn't match
        assert_eq!(
            extract_version_from_subject("Fix bug in feature"),
            None
        );
    }
}
