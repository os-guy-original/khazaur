use crate::ui;
use crate::error::Result;
use colored::Colorize;

pub fn show_history(limit: usize) -> Result<()> {
    println!("{}", ui::section_header("Operation History"));
    
    let history = crate::history::get_history(limit)?;
    
    if history.is_empty() {
        println!("{}", ui::info("No history found."));
        return Ok(());
    }
    
    for entry in history {
        let timestamp = match chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
             Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
             Err(_) => entry.timestamp.clone(),
        };
        
        let status = if entry.success {
            "SUCCESS".green()
        } else {
            "FAILED".red()
        };
        
        println!("{} [{}] {} : {}", 
            timestamp.dimmed(),
            status,
            entry.action.bold(),
            entry.packages.join(", ")
        );
    }
    
    Ok(())
}
