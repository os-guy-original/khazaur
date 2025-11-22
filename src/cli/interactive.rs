use crate::config::Config;
use crate::error::Result;
use crate::ui;
use dialoguer::{theme::ColorfulTheme, Input};
use tracing::info;

/// Interactive search using skim (fuzzy finder)
pub async fn search_interactive(_config: &mut Config) -> Result<()> {
    info!("Starting interactive search...");
    
    println!("{}", ui::section_header("Interactive Package Search"));
    
    // Prompt for search query
    let query: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Search for packages")
        .interact_text()?;
    
    if query.is_empty() {
        return Ok(());
    }
    
    // Perform search with the query
    crate::cli::search::search(&query, _config, false, false, false, false, false, false, false).await?;
    
    Ok(())
}
