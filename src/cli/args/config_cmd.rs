use crate::ui;
use crate::config::Config;
use crate::error::Result;
use clap::Subcommand;


#[derive(Subcommand, Debug, Clone)]
pub enum ConfigSubcommand {
    /// List all configuration values
    List,
    /// Get a specific configuration value
    Get { key: String },
    /// Set a configuration value
    Set { key: String, value: String },
}

pub fn handle_config(cmd: &ConfigSubcommand) -> Result<()> {
    let mut config = Config::load()?;
    let path = Config::config_file_path()?;

    match cmd {
        ConfigSubcommand::List => {
             println!("{}", ui::section_header("Current Configuration"));
             println!("File: {:?}", path);
             println!();
             
             // Manually print fields for now, or use debug print
             println!("  {}: {}", "clone_dir", config.clone_dir.display());
             println!("  {}: {}", "max_concurrent_requests", config.max_concurrent_requests);
             println!("  {}: {}", "request_delay_ms", config.request_delay_ms);
             println!("  {}: {:?}", "default_editor", config.default_editor);
             println!("  {}: {}", "confirm", config.confirm);
             println!("  {}: {}", "review_pkgbuild", config.review_pkgbuild);
        },
        ConfigSubcommand::Get { key } => {
             // Handle keys
             let value = match key.as_str() {
                 "clone_dir" => Some(config.clone_dir.display().to_string()),
                 "max_concurrent_requests" => Some(config.max_concurrent_requests.to_string()),
                 "request_delay_ms" => Some(config.request_delay_ms.to_string()),
                 "default_editor" => Some(format!("{:?}", config.default_editor)),
                 "confirm" => Some(config.confirm.to_string()),
                 "review_pkgbuild" => Some(config.review_pkgbuild.to_string()),
                 _ => None,
             };
             
             if let Some(v) = value {
                 println!("{}", v);
             } else {
                 eprintln!("{}", ui::error(&format!("Unknown config key: {}", key)));
             }
        },
        ConfigSubcommand::Set { key, value } => {
             match key.as_str() {
                 "clone_dir" => {
                     config.clone_dir = std::path::PathBuf::from(value);
                 },
                 "max_concurrent_requests" => {
                     if let Ok(v) = value.parse() {
                         config.max_concurrent_requests = v;
                     } else {
                         return Err(crate::error::KhazaurError::Config("Invalid number for max_concurrent_requests".into()));
                     }
                 },
                 "request_delay_ms" => {
                     if let Ok(v) = value.parse() {
                         config.request_delay_ms = v;
                     } else {
                         return Err(crate::error::KhazaurError::Config("Invalid number for request_delay_ms".into()));
                     }
                 },
                 "default_editor" => {
                     config.default_editor = if value.is_empty() { None } else { Some(value.clone()) };
                 },
                 "confirm" => {
                     if let Ok(v) = value.parse() {
                         config.confirm = v;
                     } else {
                         return Err(crate::error::KhazaurError::Config("Invalid boolean for confirm".into()));
                     }
                 },
                 "review_pkgbuild" => {
                     if let Ok(v) = value.parse() {
                         config.review_pkgbuild = v;
                     } else {
                         return Err(crate::error::KhazaurError::Config("Invalid boolean for review_pkgbuild".into()));
                     }
                 },
                 _ => {
                     eprintln!("{}", ui::error(&format!("Unknown config key: {}", key)));
                     return Ok(());
                 }
             }
             
             config.save()?;
             println!("{}", ui::success(&format!("Set '{}' to '{}'", key, value)));
        }
    }
    
    Ok(())
}
