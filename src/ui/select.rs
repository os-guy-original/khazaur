use crate::cli::PackageCandidate;
use crate::error::Result;
use colored::Colorize;
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
        
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Select package source for '{}'", package_name))
        .items(&items)
        .default(0)
        .interact_opt()?;
        
    Ok(selection)
}
