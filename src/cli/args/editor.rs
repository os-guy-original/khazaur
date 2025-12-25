use crate::ui;
use crate::config::Config;
use crate::error::Result;
use std::process::Command;

pub fn set_default_editor(editor_arg: &str, config: &mut Config) -> Result<()> {
    // If empty string, show interactive selection
    let editor = if editor_arg.is_empty() {
        let editors = ui::detect_editors();
        
        if editors.is_empty() {
            println!("{}", ui::error("No editors found on system"));
            return Ok(());
        }

        match ui::select_editor(&editors)? {
            Some(selected) => selected.command,
            None => {
                println!("{}", ui::warning("No editor selected"));
                return Ok(());
            }
        }
    } else {
        editor_arg.to_string()
    };
    
    // Verify editor exists
    let editor_cmd = editor.split_whitespace().next().unwrap_or(&editor);
    let exists = Command::new("which")
        .arg(editor_cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !exists {
        println!("{} {}", ui::error("Editor not found:"), editor);
        println!("Make sure '{}' is installed and in your PATH", editor_cmd);
        return Ok(());
    }

    config.default_editor = Some(editor.to_string());
    config.save()?;
    
    println!("{}", ui::success(&format!("Default editor set to: {}", editor)));
    println!("Config saved to: {:?}", Config::config_file_path()?);
    Ok(())
}
