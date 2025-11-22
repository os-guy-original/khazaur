use crate::error::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect};

/// Prompt for yes/no confirmation
pub fn confirm(message: &str, default: bool) -> Result<bool> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .default(default)
        .interact()?;
    
    Ok(result)
}

/// Prompt for text input
pub fn input(message: &str) -> Result<String> {
    let result = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .interact_text()?;
    
    Ok(result)
}

/// Prompt for multi-select from list
pub fn multi_select(message: &str, items: &[String]) -> Result<Vec<usize>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(message)
        .items(items)
        .interact()?;
    
    Ok(selections)
}

/// Prompt to select packages from a list
pub fn select_packages(packages: &[String]) -> Result<Vec<String>> {
    let indices = multi_select("Select packages to install", packages)?;
    
    let selected: Vec<String> = indices
        .iter()
        .filter_map(|&i| packages.get(i).cloned())
        .collect();
    
    Ok(selected)
}
