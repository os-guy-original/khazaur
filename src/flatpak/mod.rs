use std::process::Command;

pub mod types;
pub mod search;
pub mod install;
pub mod updates;
pub mod remotes;

// Re-export specific items for easier access
pub use types::FlatpakPackage;
pub use search::search_flatpak;
pub use install::{install_flatpak, get_installed_flatpaks, uninstall_flatpak};
pub use updates::{update_all, get_updates};

/// Check if flatpak is installed
pub fn is_available() -> bool {
    Command::new("which")
        .arg("flatpak")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}