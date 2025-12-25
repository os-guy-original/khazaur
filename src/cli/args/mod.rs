use crate::config::Config;
use crate::error::{KhazaurError, Result};
use crate::pacman;
use crate::ui;
use clap::{Parser, Subcommand};
use colored::Colorize;

pub mod remove;
pub mod query;
pub mod clean;
pub mod editor;
pub mod completions;
pub mod orphans;
pub mod health;
pub mod tree;
pub mod config_cmd;
pub mod history_cmd;
pub mod mirrors;
pub mod backup;
pub mod downgrade;
pub mod repo;

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

    /// Clean package cache (-c for khazaur only, -cc for khazaur + pacman)
    #[arg(short = 'c', action = clap::ArgAction::Count)]
    pub clean: u8,

    /// Build package from directory containing PKGBUILD (-B)
    #[arg(short = 'B', long = "build")]
    pub build: bool,

    /// Download PKGBUILD for AUR package(s) (-G)
    #[arg(short = 'G', long = "getpkgbuild")]
    pub getpkgbuild: bool,

    /// Show AUR package information (-P)
    #[arg(short = 'P', long = "show")]
    pub show: bool,

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
    /// Remove orphaned packages (unused dependencies)
    Orphans,
    /// Run a system health check
    Health,
    /// Show dependency tree for a package
    Tree {
        /// Package name
        package: String,
        /// Show GUI window
        #[arg(long)]
        gui: bool,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        cmd: config_cmd::ConfigSubcommand,
    },
    /// View operation history
    History {
        /// Number of recent entries to show
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,

    },
    /// Manage package mirrors
    Mirrors {
        /// Country/Region to use (optional)
        #[arg(long)]
        country: Option<String>,
        /// Sort by rate (download speed)
        #[arg(long)]
        fast: bool,
    },
    /// Backup or Restore package list
    Backup {
        /// Path to backup file
        path: std::path::PathBuf,
        /// Restore from backup instead of creating one
        #[arg(long)]
        restore: bool,
    },
    Downgrade {
        /// Package name
        package: String,
    },
    /// Manage package repositories
    Repo {
        /// Action to perform: list, add, remove
        #[command(subcommand)]
        action: MakeRepoCommand,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum MakeRepoCommand {
    /// List configured repositories
    List,
    /// Add a new repository
    Add,
    /// Remove a repository
    Remove,
}

impl Args {
    pub async fn execute(&self) -> Result<()> {
        // Handle --completions flag first (doesn't need config)
        if let Some(ref shell) = self.completions {
            return completions::generate_completions(shell);
        }

        // Initialize config and ensure directories exist
        let mut config = Config::load()?;
        config.ensure_dirs()?;

        // Handle --set-editor flag
        if let Some(ref editor) = self.set_editor {
            return editor::set_default_editor(editor, &mut config);
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
            return remove::remove_packages(&self.packages);
        }

        // -U: Install local package
        if let Some(ref file) = self.upgrade_local {
            return self.install_local(file);
        }

        // -Q: Query installed packages
        if self.query {
            return query::query_packages();
        }

        // -Sc or -Scc: Clean caches (with -S flag)
        if self.sync && self.clean > 0 {
            return clean::clean_cache(self.clean);
        }

        // Standalone -c or -cc: Clean caches
        if self.clean > 0 {
            return clean::clean_cache(self.clean);
        }

        // -B: Build package from directory
        if self.build {
            let dir = if self.packages.is_empty() {
                ".".to_string()
            } else {
                self.packages[0].clone()
            };
            return self.build_package(&dir);
        }

        // -G: Download PKGBUILD for AUR package(s)
        if self.getpkgbuild {
            if self.packages.is_empty() {
                return Err(KhazaurError::Config("Package name(s) required".to_string()).into());
            }
            return self.get_pkgbuild(&self.packages, &mut config).await;
        }

        // -P: Show AUR package information
        if self.show {
            if self.packages.is_empty() {
                return Err(KhazaurError::Config("Package name(s) required".to_string()).into());
            }
            return self.show_aur_packages(&self.packages, &mut config).await;
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
            Command::Orphans => orphans::clean_orphans(),
            Command::Health => health::check_health(),
            Command::Tree { package, gui } => tree::show_tree(package.clone(), *gui),
            Command::Config { cmd } => config_cmd::handle_config(cmd),
            Command::History { limit } => history_cmd::show_history(*limit),
            Command::Mirrors { country, fast } => mirrors::update_mirrors(country.clone(), *fast),
            Command::Backup { path, restore } => if *restore { 
                backup::restore(path).await 
            } else { 
                backup::backup(path) 
            },
            Command::Downgrade { package } => downgrade::downgrade(package).await,
            Command::Repo { action } => repo::handle_repo_command(action).await,
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



    fn install_local(&self, file: &str) -> Result<()> {
        println!("{}", ui::section_header("Installing Local Package"));
        pacman::install_local_package(file, &Vec::new())?;
        println!("{}", ui::success("Package installed successfully"));
        Ok(())
    }





    fn build_package(&self, dir: &str) -> Result<()> {
        use std::path::Path;
        
        println!("{}", ui::section_header("Building AUR Package"));
        
        let pkg_dir = Path::new(dir);
        let pkgbuild = pkg_dir.join("PKGBUILD");
        
        if !pkgbuild.exists() {
            return Err(KhazaurError::Config(format!(
                "PKGBUILD not found in '{}'. Use -G to download one first.",
                dir
            )).into());
        }
        
        println!("{}", ui::info(&format!("Building from: {:?}", pkg_dir.canonicalize().unwrap_or(pkg_dir.to_path_buf()))));
        
        // Build and install using makepkg
        crate::build::build_and_install(pkg_dir, true)?;
        
        println!("\n{}", ui::success("Package built and installed successfully"));
        Ok(())
    }

    async fn get_pkgbuild(&self, packages: &[String], config: &mut Config) -> Result<()> {
        use crate::aur::AurClient;
        use std::path::PathBuf;
        
        println!("{}", ui::section_header("Downloading PKGBUILD(s)"));
        
        // Check if last argument is a path (contains /, starts with ~, or is . or ..)
        let (pkg_names, output_dir): (&[String], PathBuf) = if packages.len() > 1 {
            let last = &packages[packages.len() - 1];
            if last.contains('/') || last.starts_with('~') || last == "." || last == ".." {
                // Expand ~ to home directory
                let expanded = if last.starts_with('~') {
                    dirs::home_dir()
                        .map(|h| h.join(&last[2..]))  // Skip "~/"
                        .unwrap_or_else(|| PathBuf::from(last))
                } else {
                    PathBuf::from(last)
                };
                
                // Create directory if it doesn't exist
                if !expanded.exists() {
                    std::fs::create_dir_all(&expanded)?;
                }
                
                (&packages[..packages.len() - 1], expanded)
            } else {
                (packages, std::env::current_dir()?)
            }
        } else {
            (packages, std::env::current_dir()?)
        };
        
        let client = AurClient::with_rate_limit(config.max_concurrent_requests, config.request_delay_ms)?;
        
        for pkg_name in pkg_names {
            println!("\n{}", ui::info(&format!("Downloading: {}", pkg_name)));
            
            // Check if package exists in AUR
            let results = client.search(pkg_name).await?;
            let exact_match = results.iter().find(|p| p.name == *pkg_name);
            
            if exact_match.is_none() {
                eprintln!("{}", ui::warning(&format!("Package '{}' not found in AUR", pkg_name)));
                continue;
            }
            
            // Download to output directory
            let target_dir = output_dir.join(pkg_name);
            
            if target_dir.exists() {
                println!("{}", ui::warning(&format!("Directory '{}' already exists, skipping", pkg_name)));
                continue;
            }
            
            // Clone the AUR repo
            let url = format!("https://aur.archlinux.org/{}.git", pkg_name);
            
            match git2::Repository::clone(&url, &target_dir) {
                Ok(_) => {
                    println!("{}", ui::success(&format!("Downloaded to: {:?}", target_dir)));
                }
                Err(e) => {
                    eprintln!("{}", ui::error(&format!("Failed to download '{}': {}", pkg_name, e)));
                }
            }
        }
        
        println!("\n{}", ui::success("PKGBUILD download complete"));
        Ok(())
    }

    async fn show_aur_packages(&self, packages: &[String], config: &mut Config) -> Result<()> {
        use crate::aur::AurClient;
        
        println!("{}", ui::section_header("AUR Package Information"));
        
        let client = AurClient::with_rate_limit(config.max_concurrent_requests, config.request_delay_ms)?;
        
        for pkg_name in packages {
            match client.info(pkg_name).await {
                Ok(pkg) => {
                    println!("\n{} {}", "::".bright_cyan().bold(), pkg.name.bold());
                    println!("  {} {}", "Version:".dimmed(), pkg.version);
                    
                    if let Some(desc) = &pkg.description {
                        println!("  {} {}", "Description:".dimmed(), desc);
                    }
                    
                    if let Some(url) = &pkg.url {
                        println!("  {} {}", "Upstream URL:".dimmed(), url);
                    }
                    
                    println!("  {} https://aur.archlinux.org/packages/{}", "AUR URL:".dimmed(), pkg.name);
                    
                    if let Some(maintainer) = &pkg.maintainer {
                        println!("  {} {}", "Maintainer:".dimmed(), maintainer);
                    }
                    
                    println!("  {} {}", "Votes:".dimmed(), pkg.num_votes);
                    println!("  {} {:.2}", "Popularity:".dimmed(), pkg.popularity);
                    
                    if let Some(ood) = pkg.out_of_date {
                        if ood > 0 {
                            println!("  {} {}", "Out of Date:".dimmed(), "Yes".red());
                        }
                    }
                    
                    if !pkg.depends.is_empty() {
                        println!("  {} {}", "Depends:".dimmed(), pkg.depends.join(", "));
                    }
                    
                    if !pkg.make_depends.is_empty() {
                        println!("  {} {}", "Make Depends:".dimmed(), pkg.make_depends.join(", "));
                    }
                    
                    if !pkg.opt_depends.is_empty() {
                        println!("  {} {}", "Optional Deps:".dimmed(), pkg.opt_depends.join(", "));
                    }
                }
                Err(e) => {
                    // AUR might return error if package not found
                    eprintln!("{}", ui::warning(&format!("Package '{}' not found or error: {}", pkg_name, e)));
                }
            }
        }
        
        Ok(())
    }
}
