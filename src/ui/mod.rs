mod format;
mod progress;
mod viewer;
mod editor;
mod select;

pub use format::*;
pub use progress::*;
pub use viewer::*;
pub use editor::*;
pub use select::*;

use colored::Colorize;

/// Display a section header
pub fn section_header(text: &str) -> String {
    format!("\n{}\n{}", text.bright_cyan().bold(), "═".repeat(text.len()))
}

/// Display an error message
pub fn error(text: &str) -> String {
    format!("{} {}", "✗".red().bold(), text.red())
}

/// Display a success message
pub fn success(text: &str) -> String {
    format!("{} {}", "✓".green().bold(), text.green())
}

/// Display an info message
pub fn info(text: &str) -> String {
    format!("{} {}", "→".bright_blue().bold(), text)
}

/// Display a warning message
pub fn warning(text: &str) -> String {
    format!("{} {}", "⚠".bright_yellow().bold(), text.yellow())
}
