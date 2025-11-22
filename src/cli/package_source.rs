use crate::aur::AurPackage;
use crate::flatpak::FlatpakPackage;
use crate::snap::SnapPackage;
use crate::debian::DebianPackage;
use colored::Colorize;

/// Represents a package found in a specific source
#[derive(Debug, Clone)]
pub struct PackageCandidate {
    #[allow(dead_code)]
    pub name: String,
    pub source: PackageSource,
}

/// Different sources where a package can be found
#[derive(Debug, Clone)]
pub enum PackageSource {
    /// Official repository
    Repo(crate::pacman::RepoPackage),
    /// Arch User Repository
    Aur(AurPackage),
    /// Flatpak application
    Flatpak(FlatpakPackage),
    /// Snap package
    Snap(SnapPackage),
    /// Debian package
    Debian(DebianPackage),
}

impl PackageSource {
    /// Get the source type as a string
    pub fn source_type(&self) -> &str {
        match self {
            PackageSource::Repo(_) => "repository",
            PackageSource::Aur(_) => "AUR",
            PackageSource::Flatpak(_) => "Flatpak",
            PackageSource::Snap(_) => "Snap",
            PackageSource::Debian(_) => "Debian",
        }
    }
    
    pub fn display_name(&self) -> String {
        match self {
            PackageSource::Repo(pkg) => {
                let status = if pkg.installed { " [installed]" } else { "" };
                format!("{}/{} {}{}", pkg.repository, pkg.name, pkg.version, status.bright_green())
            },
            PackageSource::Aur(pkg) => format!("aur/{} {}", pkg.name, pkg.version),
            PackageSource::Flatpak(pkg) => format!("flatpak/{} {}", pkg.name, pkg.version),
            PackageSource::Snap(pkg) => format!("snap/{} {}", pkg.name, pkg.version),
            PackageSource::Debian(pkg) => format!("debian/{} {}", pkg.name, pkg.version),
        }
    }
    
    pub fn description(&self) -> Option<&str> {
        match self {
            PackageSource::Aur(pkg) => pkg.description.as_deref(),
            PackageSource::Flatpak(pkg) => Some(&pkg.description),
            PackageSource::Snap(pkg) => Some(&pkg.description),
            PackageSource::Repo(pkg) => Some(&pkg.description),
            PackageSource::Debian(pkg) => Some(&pkg.description),
        }
    }
}
