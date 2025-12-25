use std::path::Path;
use std::process::Command;
use regex::Regex;
use crate::error::{KhazaurError, Result};

/// Check if the output from makepkg contains PGP-related errors
pub fn has_pgp_error(output: &str) -> bool {
    let pgp_patterns = [
        "PGP signature verification failed",
        "One or more PGP signatures could not be verified",
        "Could not import PGP key for verification",
        "gpg: Can't check signature",
        "gpg: keyserver receive failed",
        "gpg: keyserver timeout",
        "gpg: keyserver error",
        "invalid or corrupted packet",
        "no valid OpenPGP data found",
        "the signature could not be verified",
        "error: signature from",
        "error: failed to verify",
    ];

    pgp_patterns.iter().any(|pattern| output.contains(pattern))
}

/// Extract PGP key IDs from PKGBUILD file
pub fn extract_pgp_keys_from_pkgbuild(pkgbuild_path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(pkgbuild_path)
        .map_err(|e| KhazaurError::PgpKeyError(format!("Failed to read PKGBUILD: {}", e)))?;

    let mut keys = Vec::new();
    
    // Look for validpgpkeys array in PKGBUILD
    let re = Regex::new(r#"validpgpkeys=\(\s*([^\)]+)\s*\)"#)
        .map_err(|e| KhazaurError::PgpKeyError(format!("Regex error: {}", e)))?;

    if let Some(captures) = re.captures(&content) {
        let keys_str = &captures[1];
        
        // Extract individual keys (they're usually quoted and on separate lines)
        let key_re = Regex::new(r#""([^"]+)""#)
            .map_err(|e| KhazaurError::PgpKeyError(format!("Regex error: {}", e)))?;
        
        for cap in key_re.captures_iter(keys_str) {
            if let Some(key) = cap.get(1) {
                keys.push(key.as_str().to_string());
            }
        }
    }

    Ok(keys)
}

/// Import PGP keys using gpg
pub fn import_pgp_keys(keys: &[String]) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }

    println!("Importing missing PGP keys...");

    for key in keys {
        println!("Importing key: {}", key);

        let output = Command::new("gpg")
            .args(&["--keyserver", "keyserver.ubuntu.com", "--recv-keys", key])
            .output()
            .map_err(|e| KhazaurError::PgpKeyError(format!("Failed to run gpg command: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: Failed to import key {}: {}", key, stderr);
            
            // Try alternative keyserver
            println!("Trying alternative keyserver...");
            let output = Command::new("gpg")
                .args(&["--keyserver", "pgp.mit.edu", "--recv-keys", key])
                .output()
                .map_err(|e| KhazaurError::PgpKeyError(format!("Failed to run gpg command: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(KhazaurError::PgpKeyError(format!("Failed to import key {} from any keyserver: {}", key, stderr)));
            }
        }
    }

    Ok(())
}

/// Handle PGP key error by extracting keys from PKGBUILD and importing them
pub fn handle_pgp_error(output: &str, package_dir: &Path) -> Result<()> {
    println!("PGP signature verification failed. Attempting to import missing keys...");
    
    let pkgbuild_path = package_dir.join("PKGBUILD");
    if !pkgbuild_path.exists() {
        return Err(KhazaurError::PgpKeyError("PKGBUILD not found".to_string()));
    }

    // Extract PGP keys from PKGBUILD
    let keys = extract_pgp_keys_from_pkgbuild(&pkgbuild_path)?;
    
    if keys.is_empty() {
        return Err(KhazaurError::PgpKeyError("No validpgpkeys found in PKGBUILD".to_string()));
    }

    println!("Found {} PGP key(s) in PKGBUILD", keys.len());
    
    // Import the keys
    import_pgp_keys(&keys)?;
    
    println!("PGP keys imported successfully. Retrying build...");
    
    Ok(())
}