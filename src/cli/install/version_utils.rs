use crate::error::Result;
use std::process::Command;

/// Check if a package needs an update by comparing versions
pub fn needs_update(installed_version: &str, aur_version: &str) -> Result<bool> {
    let output = Command::new("vercmp")
        .arg(installed_version)
        .arg(aur_version)
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // vercmp returns:
    // -1 if installed < aur (update needed)
    //  0 if installed == aur (no update)
    //  1 if installed > aur (downgrade, no update)
    Ok(result == "-1")
}