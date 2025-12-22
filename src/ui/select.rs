use crate::cli::PackageCandidate;
use crate::error::Result;
use colored::Colorize;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

/// Display package candidates and let user select which source to use
pub fn select_package_source(package_name: &str, candidates: &[PackageCandidate]) -> Result<Option<usize>> {
    if candidates.is_empty() {
        return Ok(None);
    }
    
    // If only one source, return it directly
    if candidates.len() == 1 {
        return Ok(Some(0));
    }
    
    // Check if all candidates are from the same source type
    let first_source_type = candidates[0].source.source_type();
    let all_same_source = candidates.iter().all(|c| c.source.source_type() == first_source_type);
    
    if all_same_source {
        println!("\n{}", format!("Multiple '{}' packages found matching '{}':", first_source_type, package_name).bold());
    } else {
        println!("\n{}", format!("Package '{}' found in multiple sources:", package_name).bold());
    }
    println!();
    
    let items: Vec<String> = candidates
        .iter()
        .enumerate()
        .map(|(i, candidate)| {
            let mut item = format!("{}. {}", i + 1, candidate.source.display_name());
            if let Some(desc) = candidate.source.description() {
                item.push_str(&format!("\n   {}", desc.dimmed()));
            }
            item
        })
        .collect();
    
    // Calculate max visible items based on terminal height
    // Reserve lines for: prompt (1), header text (3), footer/scroll indicator (2), some padding (2)
    let max_height = get_terminal_max_items(8);
    
    let theme = ColorfulTheme::default();
    let mut select = Select::with_theme(&theme)
        .with_prompt(format!("Select package source for '{}' (↑/↓ to scroll, Enter to select)", package_name))
        .items(&items)
        .default(0);
    
    // Only apply max_length if we have more items than can fit
    if items.len() > max_height {
        select = select.max_length(max_height);
    }
    
    let selection = select.interact_opt()?;
        
    Ok(selection)
}

/// Get the maximum number of items that can be displayed in the terminal
/// Returns a reasonable default if terminal size cannot be determined
fn get_terminal_max_items(reserved_lines: usize) -> usize {
    // Try to get terminal height
    let term = Term::stderr();
    let height = term.size().0 as usize;
    
    // Default to 20 if we can't get terminal size or it's too small
    if height <= reserved_lines {
        return 15;
    }
    
    // Calculate available lines and ensure a reasonable minimum
    let available = height.saturating_sub(reserved_lines);
    
    // For items with descriptions (2 lines each), divide by 2
    // But ensure at least 5 items are visible
    let max_items = available / 2;
    max_items.max(5)
}
