mod data;
mod tui;
mod gui;

use crate::error::Result;

pub fn show_tree(package: String, use_gui: bool) -> Result<()> {
    // If GUI requested
    if use_gui {
        gui::run(&package)?;
    } else {
        tui::run(&package)?;
    }
    Ok(())
}
