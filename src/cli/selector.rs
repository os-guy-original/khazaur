//! Reusable selector module for interactive item selection
//! Uses the same style as the package source selector

use crate::error::Result;
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

/// A selectable item with name and description (shown on second line)
pub struct SelectItem {
    pub name: String,
    pub description: Option<String>,
}

impl SelectItem {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }
    
    pub fn with_desc(name: impl Into<String>, desc: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(desc.into()),
        }
    }
}

/// Display items and let user select one
/// Uses the same visual style as the package source selector
pub fn select_items(
    prompt: &str,
    header: Option<&str>,
    items: &[SelectItem],
) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }
    
    if let Some(hdr) = header {
        println!("\n{}", hdr.bold());
        println!();
    }
    
    // Format items with numbers and optional description on second line
    let display_items: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut line = format!("{}. {}", i + 1, item.name);
            if let Some(desc) = &item.description {
                line.push_str(&format!("\n   {}", desc.dimmed()));
            }
            line
        })
        .collect();
    
    // Calculate max visible items based on terminal height
    let max_height = get_terminal_max_items(8);
    
    let theme = ColorfulTheme::default();
    let mut select = Select::with_theme(&theme)
        .with_prompt(prompt)
        .items(&display_items)
        .default(0);
    
    // Only apply max_length if we have more items than can fit
    if display_items.len() > max_height {
        select = select.max_length(max_height);
    }
    
    let selection = select.interact_opt()?;
    
    Ok(selection)
}

/// Convenience function: display simple string items without descriptions
pub fn select_string(
    prompt: &str,
    items: &[String],
    _show_cancel: bool,
) -> Result<Option<usize>> {
    let item_list: Vec<SelectItem> = items
        .iter()
        .map(|s| SelectItem::new(s.clone()))
        .collect();
    
    select_items(prompt, None, &item_list)
}

/// Get the maximum number of items that can be displayed in the terminal
fn get_terminal_max_items(reserved_lines: usize) -> usize {
    let term = Term::stderr();
    let height = term.size().0 as usize;
    
    if height <= reserved_lines {
        return 15;
    }
    
    let available = height.saturating_sub(reserved_lines);
    
    // For items with descriptions (2 lines each), divide by 2
    // But ensure at least 5 items are visible
    let max_items = available / 2;
    max_items.max(5)
}
