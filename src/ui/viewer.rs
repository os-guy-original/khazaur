use crate::config::Config;
use crate::error::Result;
use crate::ui::editor;
use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Display PKGBUILD with a "press key to view" prompt (pacman-style)
pub fn view_pkgbuild_interactive(
    pkgbuild_path: &Path,
    config: &mut Config,
) -> Result<bool> {
    // Read current PKGBUILD content
    let pkgbuild_content = fs::read_to_string(pkgbuild_path)?;
    
    println!("\n{} {}", "::".bright_blue().bold(), "PKGBUILD Review".bold());
    print!("   {} ", "Press [V]iew, [E]dit, or [S]kip:".white());
    io::stdout().flush()?;

    // Read single character
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    match input.as_str() {
        "v" | "view" => {
            // Display PKGBUILD content
            println!("\n{}", pkgbuild_content);
            println!("\n{} {}", "::".bright_blue().bold(), "End of PKGBUILD".bold());
            
            // Ask to continue
            print!("   {} ", "Continue with build? [Y/n]:".white());
            io::stdout().flush()?;
            
            let mut continue_input = String::new();
            io::stdin().read_line(&mut continue_input)?;
            let continue_input = continue_input.trim().to_lowercase();
            
            Ok(continue_input != "n" && continue_input != "no")
        }
        "e" | "edit" => {
            loop {
                // Get editor to use
                let editor_cmd = if let Some(ref default_editor) = config.default_editor {
                    default_editor.clone()
                } else {
                    // Detect available editors
                    let editors = editor::detect_editors();
                    
                    if editors.is_empty() {
                        println!("   {}", "No editors found on system".red());
                        print!("   {} ", "Continue with build? [Y/n]:".white());
                        io::stdout().flush()?;
                        
                        let mut continue_input = String::new();
                        io::stdin().read_line(&mut continue_input)?;
                        let continue_input = continue_input.trim().to_lowercase();
                        
                        return Ok(continue_input != "n" && continue_input != "no");
                    }

                    // Prompt user to select editor
                    match editor::select_editor(&editors)? {
                        Some(selected_editor) => {
                            // Ask if should be saved as default
                            if editor::prompt_save_default()? {
                                config.default_editor = Some(selected_editor.command.clone());
                                config.save()?;
                                println!("   {}", format!("Saved {} as default editor", selected_editor.name).green());
                            }
                            selected_editor.command
                        }
                        None => {
                            println!("   {}", "No editor selected".yellow());
                            print!("   {} ", "Continue with build? [Y/n]:".white());
                            io::stdout().flush()?;
                            
                            let mut continue_input = String::new();
                            io::stdin().read_line(&mut continue_input)?;
                            let continue_input = continue_input.trim().to_lowercase();
                            
                            return Ok(continue_input != "n" && continue_input != "no");
                        }
                    }
                };

                // Open editor
                println!("   {}", "Opening editor...".bright_blue());
                editor::open_in_editor(&editor_cmd, pkgbuild_path)?;
                
                // Reload PKGBUILD after editing
                let new_content = fs::read_to_string(pkgbuild_path)?;
                
                if new_content != pkgbuild_content {
                    println!("\n{} {}", "::".bright_yellow().bold(), "PKGBUILD was modified".bold());
                } else {
                    println!("\n{} {}", "::".bright_blue().bold(), "No changes made".bold());
                }
                
                print!("   {} ", "Continue with build? [Y/n/r] (r=re-edit):".white());
                io::stdout().flush()?;
                
                let mut continue_input = String::new();
                io::stdin().read_line(&mut continue_input)?;
                let continue_input = continue_input.trim().to_lowercase();
                
                if continue_input == "r" || continue_input == "re-edit" {
                    continue;
                }
                
                return Ok(continue_input != "n" && continue_input != "no");
            }
        }
        _ => {
            // Skip review
            Ok(true)
        }
    }
}
