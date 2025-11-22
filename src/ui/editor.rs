use crate::error::{KhazaurError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum EditorType {
    Gui,
    Tui,
    Cli,
}

#[derive(Debug, Clone)]
pub struct Editor {
    pub name: String,
    pub command: String,
    pub editor_type: EditorType,
}

impl Editor {
    fn new(name: &str, command: &str, editor_type: EditorType) -> Self {
        Self {
            name: name.to_string(),
            command: command.to_string(),
            editor_type,
        }
    }

}

/// Detect available editors on the system
pub fn detect_editors() -> Vec<Editor> {
    let mut editors = Vec::new();

    // GUI editors (with blocking support)
    let gui_editors = [
        ("VS Code", "code"),
        ("VS Code (alternate)", "vscode"),
        ("Gedit", "gedit"),
        ("Kate", "kate"),
        ("Sublime Text", "subl"),
        ("Atom", "atom"),
        ("Mousepad", "mousepad"),
    ];

    for (name, cmd) in &gui_editors {
        if command_exists(cmd) {
            editors.push(Editor::new(name, cmd, EditorType::Gui));
        }
    }

    // TUI editors
    let tui_editors = [
        ("Micro", "micro"),
        ("Nano", "nano"),
        ("Vim", "vim"),
        ("Neovim", "nvim"),
        ("Vi", "vi"),
        ("Emacs (TUI)", "emacs -nw"),
    ];

    for (name, cmd) in &tui_editors {
        let check_cmd = cmd.split_whitespace().next().unwrap();
        if command_exists(check_cmd) {
            editors.push(Editor::new(name, cmd, EditorType::Tui));
        }
    }

    // Environment variable editors
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editors.iter().any(|e| e.command == editor) {
            editors.push(Editor::new(&format!("$EDITOR ({})", editor), &editor, EditorType::Cli));
        }
    }

    if let Ok(visual) = std::env::var("VISUAL") {
        if !editors.iter().any(|e| e.command == visual) {
            editors.push(Editor::new(&format!("$VISUAL ({})", visual), &visual, EditorType::Cli));
        }
    }

    editors
}

/// Check if a command exists in PATH
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Prompt user to select an editor from available options
pub fn select_editor(editors: &[Editor]) -> Result<Option<Editor>> {
    use dialoguer::{Select, theme::ColorfulTheme};
    
    if editors.is_empty() {
        return Ok(None);
    }

    // Create formatted display names with type indicators
    let mut items: Vec<String> = Vec::new();
    let mut section_starts = Vec::new();
    
    // Group by type
    let gui_editors: Vec<_> = editors.iter().filter(|e| e.editor_type == EditorType::Gui).collect();
    let tui_editors: Vec<_> = editors.iter().filter(|e| e.editor_type == EditorType::Tui).collect();
    let cli_editors: Vec<_> = editors.iter().filter(|e| e.editor_type == EditorType::Cli).collect();

    let mut all_editors = Vec::new();

    if !gui_editors.is_empty() {
        section_starts.push((items.len(), "GUI Editors"));
        for editor in &gui_editors {
            items.push(format!("  {}", editor.name));
            all_editors.push(editor);
        }
    }

    if !tui_editors.is_empty() {
        section_starts.push((items.len(), "Terminal Editors"));
        for editor in &tui_editors {
            items.push(format!("  {}", editor.name));
            all_editors.push(editor);
        }
    }

    if !cli_editors.is_empty() {
        section_starts.push((items.len(), "Environment"));
        for editor in &cli_editors {
            items.push(format!("  {}", editor.name));
            all_editors.push(editor);
        }
    }

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an editor")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| KhazaurError::Config(format!("Selection failed: {}", e)))?;

    Ok(Some((*all_editors[selection]).clone()))
}

/// Ask if the user wants to save this as the default editor
pub fn prompt_save_default() -> Result<bool> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Save as default editor?")
        .default(true)
        .interact()
        .map_err(|e| KhazaurError::Config(format!("Confirmation failed: {}", e)))?;
    
    Ok(result)
}

/// Open a file in the specified editor
pub fn open_in_editor(editor_command: &str, file_path: &Path) -> Result<()> {
    let parts: Vec<&str> = editor_command.split_whitespace().collect();
    let (cmd, args) = parts.split_first().ok_or_else(|| {
        KhazaurError::Config("Invalid editor command".to_string())
    })?;

    // Determine if this is a GUI editor and add blocking flags
    let mut final_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    
    // Add blocking flags for known GUI editors
    let cmd_base = cmd.split('/').last().unwrap_or(cmd);
    match cmd_base {
        "code" | "vscode" => final_args.insert(0, "--wait".to_string()),
        "gedit" => final_args.insert(0, "--wait".to_string()),
        "kate" => final_args.insert(0, "--block".to_string()),
        "subl" => final_args.insert(0, "-w".to_string()),
        "atom" => final_args.insert(0, "--wait".to_string()),
        _ => {} // TUI editors and others don't need special flags
    }

    final_args.push(file_path.to_string_lossy().to_string());

    let status = Command::new(cmd)
        .args(&final_args)
        .status()?;

    if !status.success() {
        return Err(KhazaurError::Config(
            format!("Editor exited with status: {}", status)
        ));
    }

    Ok(())
}
