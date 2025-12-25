use crate::cli::Args;
use crate::ui;
use crate::error::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn generate_completions(shell: &str) -> Result<()> {
    let shell_type = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            eprintln!("{}", ui::error(&format!("Unsupported shell: {}", shell)));
            eprintln!("Supported shells: bash, zsh, fish, powershell, elvish");
            return Ok(());
        }
    };

    let mut cmd = Args::command();
    let bin_name = cmd.get_name().to_string();
    
    generate(shell_type, &mut cmd, bin_name, &mut io::stdout());
    
    Ok(())
}
