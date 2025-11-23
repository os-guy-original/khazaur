use crate::config::Config;
use crate::error::{KhazaurError, Result};
use crate::pacman;
use crate::ui;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(name = "khazaur")]
#[command(about = "A modern package manager with multi-source support (repos, AUR, Flatpak, Snap)")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Sync/Install packages (-S)
    #[arg(short = 'S', long)]
    pub sync: bool,

    /// Refresh package databases (-y)
    #[arg(short = 'y', long)]
    pub refresh: bool,

    /// Upgrade packages (-u)
    #[arg(short = 'u', long)]
    pub upgrade: bool,

    /// Search for packages (use with -S for search, e.g., -Ss query)
    #[arg(short = 's', long)]
    pub search: bool,

    /// Show package information (-i)
    #[arg(short = 'i', long)]
    pub info: bool,

    /// Remove packages (-R)
    #[arg(short = 'R', long)]
    pub remove: bool,

    /// Install local package file (-U)
    #[arg(short = 'U', long)]
    pub upgrade_local: Option<String>,

    /// Query installed packages (-Q)
    #[arg(short = 'Q', long)]
    pub query: bool,

    /// Search/install only from AUR
    #[arg(long)]
    pub aur: bool,

    /// Search/install only from official repositories
    #[arg(long)]
    pub repo: bool,

    /// Search/install only from Flatpak
    #[arg(long)]
    pub flatpak: bool,

    /// Search/install only from Snap
    #[arg(long)]
    pub snap: bool,

    /// Search/install only from Debian packages
    #[arg(long)]
    pub debian: bool,

    /// Don't ask for confirmation
    #[arg(long)]
    pub noconfirm: bool,

    /// Verbose output (show debug information)
    #[arg(short = 'v', long)]
    pub verbose: bool,

    /// Set default text editor (interactive if no editor specified)
    #[arg(long, value_name = "EDITOR", num_args = 0..=1, default_missing_value = "")]
    pub set_editor: Option<String>,

    /// Interactive fuzzy search
    #[arg(long = "interactive", short = 'I')]
    pub interactive: bool,

    /// Disable timeout for Flatpak search (may cause longer waits)
    #[arg(long)]
    pub no_timeout: bool,

    /// Generate shell completions for the specified shell
    #[arg(long = "completions", value_name = "SHELL")]
    pub completions: Option<String>,

    /// Package names or search query
    pub packages: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Search for packages
    Search {
        /// Search query
        query: String,
    },
    /// Install packages
    Install {
        /// Package names
        packages: Vec<String>,
    },
    /// Update system
    Update,
}

impl Args {
    pub async fn execute(&self) -> Result<()> {
        // Handle --completions flag first (doesn't need config)
        if let Some(ref shell) = self.completions {
            return self.generate_completions(shell);
        }

        // Initialize config and ensure directories exist
        let mut config = Config::load()?;
        config.ensure_dirs()?;

        // Handle --set-editor flag
        if let Some(ref editor) = self.set_editor {
            return self.set_default_editor(editor, &mut config);
        }

        // Handle --interactive flag
        if self.interactive {
            return crate::cli::interactive::search_interactive(&mut config).await;
        }

        // Handle subcommands first
        if let Some(ref cmd) = self.command {
            return self.execute_subcommand(cmd, &mut config).await;
        }

        // Handle flag-based commands (pacman-style)
        
        // -Syu: Sync databases and upgrade
        if self.sync && self.refresh && self.upgrade {
            return self.system_upgrade(&mut config).await;
        }

        // -Sy: Sync databases only
        if self.sync && self.refresh && self.packages.is_empty() && !self.upgrade {
            return self.sync_databases().await;
        }

        // -Ss: Search packages
        if self.sync && self.search {
            if self.packages.is_empty() {
                return Err(KhazaurError::Config("Search query required".to_string()).into());
            }
            let query = self.packages.join(" ");
            return self.search_packages(&query, &mut config).await;
        }

        // -Si: Show package info
        if self.sync && self.info && !self.packages.is_empty() {
            return self.show_package_info(&self.packages[0], &mut config).await;
        }

        // -S: Install packages
        if self.sync && !self.packages.is_empty() {
            return self.install_packages(&self.packages, &mut config).await;
        }

        // -R: Remove packages
        if self.remove && !self.packages.is_empty() {
            return self.remove_packages(&self.packages);
        }

        // -U: Install local package
        if let Some(ref file) = self.upgrade_local {
            return self.install_local(file);
        }

        // -Q: Query installed packages
        if self.query {
            return self.query_packages();
        }

        // Auto-detect .deb files for installation (if no other operation flags are set)
        if !self.sync && !self.remove && !self.query && self.upgrade_local.is_none() && !self.packages.is_empty() {
            if self.packages.iter().any(|p| p.ends_with(".deb")) {
                return self.install_packages(&self.packages, &mut config).await;
            }
        }

        // No valid command found
        println!("{}", ui::error("No valid operation specified"));
        println!("Try 'khazaur --help' for more information");
        Ok(())
    }

    async fn execute_subcommand(&self, cmd: &Command, config: &mut Config) -> Result<()> {
        match cmd {
            Command::Search { query } => self.search_packages(query, config).await,
            Command::Install { packages } => self.install_packages(packages, config).await,
            Command::Update => self.system_upgrade(config).await,
        }
    }

    async fn sync_databases(&self) -> Result<()> {
        println!("{}", ui::section_header("Syncing Package Databases"));
        
        // Determine what to sync based on flags
        let sync_all = !self.aur && !self.repo && !self.snap && !self.debian;
        
        // Sync pacman databases (repos and AUR metadata)
        if sync_all || self.repo || self.aur {
            pacman::sync_databases()?;
        }
        
        // Update Debian package index and debtap if requested
        if sync_all || self.debian {
            // Update Debian package index with progress bar
            match crate::debian::update_index().await {
                Ok(_) => {
                    println!("{}", ui::success("Debian index updated"));
                }
                Err(e) => {
                    eprintln!("{}", ui::warning(&format!("Failed to update Debian index: {}", e)));
                }
            }
            
            // Update debtap database last (takes longer)
            if crate::debtap::is_available() {
                println!("\n{}", ui::info("Updating debtap database (this may take a while)..."));
                if let Err(e) = crate::debtap::update_database() {
                    eprintln!("{}", ui::warning(&format!("Failed to update debtap database: {}", e)));
                }
            }
        }
        
        println!("\n{}", ui::success("Database sync complete"));
        Ok(())
    }

    async fn search_packages(&self, query: &str, config: &mut Config) -> Result<()> {
        crate::cli::search::search(
            query,
            config,
            self.aur,
            self.repo,
            self.aur,
            self.repo,
            self.flatpak,
            self.snap,
            self.debian,
        ).await
    }

    async fn show_package_info(&self, package_name: &str, config: &mut Config) -> Result<()> {
        crate::cli::search::show_info(package_name, config).await
    }

    async fn install_packages(&self, packages: &[String], config: &mut Config) -> Result<()> {
        crate::cli::install::install(
            packages,
            config,
            self.noconfirm,
            self.aur,
            self.repo,
            self.flatpak,
            self.snap,
            self.debian,
            self.no_timeout,
        ).await
    }

    async fn system_upgrade(&self, config: &mut Config) -> Result<()> {
        println!("{}", ui::section_header("System Upgrade"));
        
        // Sync databases first
        println!("{}", ui::info("Synchronizing package databases..."));
        pacman::sync_databases()?;
        
        // Check for all updates (repo + AUR) and upgrade together
        crate::cli::install::upgrade_system(config, self.noconfirm).await?;
        
        // Refresh snap if available
        if crate::snap::is_available() {
            println!("\n{}", ui::info("Refreshing snap packages..."));
            let status = std::process::Command::new("snap")
                .args(["refresh"])
                .status();
            
            match status {
                Ok(s) if s.success() => {
                    println!("{}", ui::success("Snap packages refreshed"));
                }
                Ok(_) => {
                    eprintln!("{}", ui::warning("Snap refresh failed"));
                }
                Err(e) => {
                    eprintln!("{}", ui::warning(&format!("Failed to run snap refresh: {}", e)));
                }
            }
        }
        
        // Update Debian package index with progress bar
        match crate::debian::update_index().await {
            Ok(_) => {
                println!("{}", ui::success("Debian index updated"));
            }
            Err(e) => {
                eprintln!("{}", ui::warning(&format!("Failed to update Debian index: {}", e)));
            }
        }
        
        // Update debtap database last (takes longer)
        if crate::debtap::is_available() {
            println!("\n{}", ui::info("Updating debtap database (this may take a while)..."));
            if let Err(e) = crate::debtap::update_database() {
                eprintln!("{}", ui::warning(&format!("Failed to update debtap database: {}", e)));
            }
        }
        
        println!("\n{}", ui::success("System upgrade complete"));
        Ok(())
    }

    fn set_default_editor(&self, editor_arg: &str, config: &mut Config) -> Result<()> {
        use std::process::Command;
        
        // If empty string, show interactive selection
        let editor = if editor_arg.is_empty() {
            let editors = ui::detect_editors();
            
            if editors.is_empty() {
                println!("{}", ui::error("No editors found on system"));
                return Ok(());
            }

            match ui::select_editor(&editors)? {
                Some(selected) => selected.command,
                None => {
                    println!("{}", ui::warning("No editor selected"));
                    return Ok(());
                }
            }
        } else {
            editor_arg.to_string()
        };
        
        // Verify editor exists
        let editor_cmd = editor.split_whitespace().next().unwrap_or(&editor);
        let exists = Command::new("which")
            .arg(editor_cmd)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !exists {
            println!("{} {}", ui::error("Editor not found:"), editor);
            println!("Make sure '{}' is installed and in your PATH", editor_cmd);
            return Ok(());
        }

        config.default_editor = Some(editor.to_string());
        config.save()?;
        
        println!("{}", ui::success(&format!("Default editor set to: {}", editor)));
        println!("Config saved to: {:?}", Config::config_file_path()?);
        Ok(())
    }

    fn remove_packages(&self, packages: &[String]) -> Result<()> {
        println!("{}", ui::section_header("Removing Packages"));
        
        let mut pacman_packages = Vec::new();
        let mut flatpak_packages = Vec::new();
        let mut snap_packages = Vec::new();
        
        for query in packages {
            // Search across all sources
            let pacman_matches = pacman::search_installed_packages(query)?;
            let flatpak_matches = if crate::flatpak::is_available() {
                crate::flatpak::get_installed_flatpaks(query)?
            } else {
                Vec::new()
            };
            let snap_matches = if crate::snap::is_available() {
                crate::snap::get_installed_snaps(query)?
            } else {
                Vec::new()
            };
            
            let total_matches = pacman_matches.len() + flatpak_matches.len() + snap_matches.len();
            
            if total_matches == 0 {
                println!("{}", ui::warning(&format!("No installed packages found matching '{}'", query)));
                continue;
            } else if total_matches == 1 {
                // Single match, add directly
                if !pacman_matches.is_empty() {
                    println!("{}", ui::info(&format!("Found (pacman): {}", pacman_matches[0])));
                    pacman_packages.push(pacman_matches[0].clone());
                } else if !flatpak_matches.is_empty() {
                    println!("{}", ui::info(&format!("Found (flatpak): {}", flatpak_matches[0])));
                    flatpak_packages.push(flatpak_matches[0].clone());
                } else {
                    println!("{}", ui::info(&format!("Found (snap): {}", snap_matches[0])));
                    snap_packages.push(snap_matches[0].clone());
                }
            } else {
                // Multiple matches, show selection UI with source indicators
                use dialoguer::{theme::ColorfulTheme, MultiSelect};
                
                println!("\n{}", ui::info(&format!("Multiple packages found matching '{}':", query)));
                
                // Build items with source indicators
                let mut items = Vec::new();
                let mut sources = Vec::new();
                
                for pkg in &pacman_matches {
                    items.push(format!("{} (pacman)", pkg));
                    sources.push(("pacman", pkg.clone()));
                }
                for pkg in &flatpak_matches {
                    items.push(format!("{} (flatpak)", pkg));
                    sources.push(("flatpak", pkg.clone()));
                }
                for pkg in &snap_matches {
                    items.push(format!("{} (snap)", pkg));
                    sources.push(("snap", pkg.clone()));
                }
                
                let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select packages to remove (Space to select, Enter to confirm)")
                    .items(&items)
                    .interact()?;
                
                if selections.is_empty() {
                    println!("{}", ui::warning("No packages selected"));
                    continue;
                }
                
                for &idx in &selections {let (source, pkg) = &sources[idx];
                    match *source {
                        "pacman" => pacman_packages.push(pkg.clone()),
                        "flatpak" => flatpak_packages.push(pkg.clone()),
                        "snap" => snap_packages.push(pkg.clone()),
                        _ => {}
                    }
                }
            }
        }
        
        let total_to_remove = pacman_packages.len() + flatpak_packages.len() + snap_packages.len();
        
        if total_to_remove == 0 {
            println!("{}", ui::warning("No packages to remove"));
            return Ok(());
        }
        
        // Show summary
        if !pacman_packages.is_empty() {
            println!("\n{}", ui::info(&format!("Pacman packages: {}", pacman_packages.join(", "))));
        }
        if !flatpak_packages.is_empty() {
            println!("{}", ui::info(&format!("Flatpak apps: {}", flatpak_packages.join(", "))));
        }
        if !snap_packages.is_empty() {
            println!("{}", ui::info(&format!("Snap packages: {}", snap_packages.join(", "))));
        }
        
        // Ask for confirmation
        use dialoguer::{theme::ColorfulTheme, Confirm};
        
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with removal?")
            .default(false)
            .interact()?;
        
        if !confirmed {
            println!("{}", ui::warning("Removal cancelled"));
            return Ok(());
        }
        
        // Remove pacman packages (pass --noconfirm since user already confirmed)
        if !pacman_packages.is_empty() {
            match pacman::remove_packages(&pacman_packages, &vec!["--noconfirm".to_string()]) {
                Ok(_) => {
                    println!("{}", ui::success("Pacman packages removed successfully"));
                },
                Err(e) => {
                    let error_msg = e.to_string();
                    
                    // Check if it's a dependency conflict
                    if error_msg.contains("dependency_conflict:") {
                        println!("\n{}", ui::warning("⚠️  Dependency conflict detected"));
                        println!("{}", ui::info("Some packages depend on the package(s) you're trying to remove."));
                        println!("{}", ui::info("You can force removal with -Rdd, but this may break dependent packages.\n"));
                        
                        use dialoguer::{theme::ColorfulTheme, Confirm};
                        
                        let force_remove = Confirm::with_theme(&ColorfulTheme::default())
                            .with_prompt("Force removal (skip dependency checks)?")
                            .default(false)
                            .interact()?;
                        
                        if force_remove {
                            println!("{}", ui::warning("Force removing packages (ignoring dependencies)..."));
                            pacman::remove_packages(&pacman_packages, &vec!["-dd".to_string(), "--noconfirm".to_string()])?;
                        } else {
                            println!("{}", ui::warning("Removal cancelled"));
                            return Ok(());
                        }
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        // Remove flatpak packages
        for app_id in &flatpak_packages {
            if let Err(e) = crate::flatpak::uninstall_flatpak(app_id) {
                eprintln!("{}", ui::error(&format!("Failed to remove flatpak {}: {}", app_id, e)));
            } else {
                println!("{}", ui::success(&format!("Removed flatpak: {}", app_id)));
            }
        }
        
        // Remove snap packages
        for pkg in &snap_packages {
            if let Err(e) = crate::snap::uninstall_snap(pkg) {
                eprintln!("{}", ui::error(&format!("Failed to remove snap {}: {}", pkg, e)));
            } else {
                println!("{}", ui::success(&format!("Removed snap: {}", pkg)));
            }
        }
        
        println!("\n{}", ui::success("Package removal complete"));
        Ok(())
    }

    fn install_local(&self, file: &str) -> Result<()> {
        println!("{}", ui::section_header("Installing Local Package"));
        pacman::install_local_package(file, &Vec::new())?;
        println!("{}", ui::success("Package installed successfully"));
        Ok(())
    }

    fn query_packages(&self) -> Result<()> {
        println!("{}", ui::section_header("Installed Packages"));
        
        // Get pacman packages (repo + AUR)
        let pacman_packages = pacman::get_installed_packages()?;
        let aur_packages = pacman::get_installed_aur_packages()?;
        
        // Create a set of AUR package names for quick lookup
        let aur_names: std::collections::HashSet<String> = aur_packages
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        
        // Separate repo and AUR packages
        let mut repo_packages = Vec::new();
        for (name, version) in &pacman_packages {
            if !aur_names.contains(name) {
                repo_packages.push((name.clone(), version.clone()));
            }
        }
        
        // Get Flatpak packages
        let flatpak_packages = if crate::flatpak::is_available() {
            crate::flatpak::get_installed_flatpaks("")?
        } else {
            Vec::new()
        };
        
        // Get Snap packages
        let snap_packages = if crate::snap::is_available() {
            crate::snap::get_installed_snaps("")?
        } else {
            Vec::new()
        };
        
        // Display summary
        let total = pacman_packages.len() + flatpak_packages.len() + snap_packages.len();
        println!("\n{} Total: {}, Repository: {}, AUR: {}, Flatpak: {}, Snap: {}\n",
            "::".bright_blue().bold(),
            total,
            repo_packages.len(),
            aur_packages.len(),
            flatpak_packages.len(),
            snap_packages.len()
        );
        
        // Display repository packages
        if !repo_packages.is_empty() {
            println!("{} {} ({})", 
                "::".bright_blue().bold(),
                "Repository Packages".bold(),
                repo_packages.len()
            );
            for (name, version) in &repo_packages {
                println!("  {} {}", name, version.dimmed());
            }
            println!();
        }
        
        // Display AUR packages
        if !aur_packages.is_empty() {
            println!("{} {} ({})", 
                "::".bright_cyan().bold(),
                "AUR Packages".bold(),
                aur_packages.len()
            );
            for (name, version) in &aur_packages {
                println!("  {} {}", name, version.dimmed());
            }
            println!();
        }
        
        // Display Flatpak packages
        if !flatpak_packages.is_empty() {
            println!("{} {} ({})", 
                "::".bright_green().bold(),
                "Flatpak Applications".bold(),
                flatpak_packages.len()
            );
            for app_id in &flatpak_packages {
                println!("  {}", app_id);
            }
            println!();
        }
        
        // Display Snap packages
        if !snap_packages.is_empty() {
            println!("{} {} ({})", 
                "::".bright_yellow().bold(),
                "Snap Packages".bold(),
                snap_packages.len()
            );
            for name in &snap_packages {
                println!("  {}", name);
            }
            println!();
        }
        
        Ok(())
    }

    fn generate_completions(&self, shell: &str) -> Result<()> {
        use clap::CommandFactory;
        use clap_complete::{generate, Shell};
        use std::io;

        let shell_type = match shell.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
                eprintln!("{}", ui::error(&format!("Unsupported shell: {}", shell)));
                eprintln!("Supported shells: bash, zsh, fish, powershell, elvish");
                return Ok(());
            }
        };

        let mut cmd = Args::command();
        let bin_name = cmd.get_name().to_string();
        
        generate(shell_type, &mut cmd, bin_name, &mut io::stdout());
        
        Ok(())
    }
}
