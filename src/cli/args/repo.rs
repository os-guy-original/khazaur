use crate::error::Result;
use crate::ui;
use super::MakeRepoCommand;
use dialoguer::{theme::ColorfulTheme, Select, Input, Confirm, FuzzySelect};
use colored::Colorize;

pub async fn handle_repo_command(action: &MakeRepoCommand) -> Result<()> {
    match action {
        MakeRepoCommand::List => list_repos().await,
        MakeRepoCommand::Add => add_repo().await,
        MakeRepoCommand::Remove => remove_repo().await,
    }
}

async fn list_repos() -> Result<()> {
    println!("{}", ui::section_header("Configured Repositories"));

    // 1. List Pacman Repos
    println!("\n{}", ":: Pacman Repositories ::".bright_blue().bold());
    match crate::pacman::repos::list_repos() {
        Ok(repos) => {
            if repos.is_empty() {
                println!("  {}", "No custom repositories found (standard ones assumed)".dimmed());
            } else {
                for repo in repos {
                    println!("  {} ({})", repo.name.bold(), repo.url.dimmed());
                }
            }
        }
        Err(e) => eprintln!("  {}", ui::warning(&format!("Failed to list pacman repos: {}", e))),
    }

    // 2. List Flatpak Remotes
    if crate::flatpak::is_available() {
        println!("\n{}", ":: Flatpak Remotes ::".bright_blue().bold());
        match crate::flatpak::remotes::list_remotes() {
            Ok(remotes) => {
                if remotes.is_empty() {
                    println!("  {}", "No remotes configured".dimmed());
                } else {
                    for remote in remotes {
                        println!("  {} ({})", remote.name.bold(), remote.url.dimmed());
                    }
                }
            }
            Err(e) => eprintln!("  {}", ui::warning(&format!("Failed to list flatpak remotes: {}", e))),
        }
    }

    Ok(())
}

async fn add_repo() -> Result<()> {
    println!("{}", ui::section_header("Add Repository"));

    let types = vec!["Pacman (Arch Linux)", "Flatpak Remote"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select repository type")
        .default(0)
        .items(&types)
        .interact()?;

    match selection {
        0 => add_pacman_repo().await,
        1 => add_flatpak_remote().await,
        _ => Ok(()),
    }
}

async fn add_pacman_repo() -> Result<()> {
    let methods = vec!["Browse Suggested Repos (Arch Wiki)".to_string(), "Enter Manually".to_string()];
    let selection = crate::cli::selector::select_string("How do you want to add the repository?", &methods, true)?;

    let (name, url) = if selection == Some(0) {
        // Browse Suggested
        println!("{}", ui::info("Fetching suggested repos from Arch Wiki..."));
        match crate::pacman::repos::fetch_suggested_repos().await {
            Ok(suggestions) => {
                let suggestions: Vec<crate::pacman::repos::SuggestedRepo> = suggestions;
                if suggestions.is_empty() {
                    println!("{}", ui::warning("No suggestions found"));
                    return Ok(());
                }

                let items: Vec<crate::cli::selector::SelectItem> = suggestions.iter()
                    .map(|r| crate::cli::selector::SelectItem::with_desc(
                        format!("{} - {}", r.name, r.description),
                        &r.server
                    ))
                    .collect();

                match crate::cli::selector::select_items("Select a repository", None, &items)? {
                    Some(idx) => {
                        let selected = &suggestions[idx];
                        (selected.name.clone(), selected.server.clone())
                    }
                    None => {
                        println!("{}", ui::warning("Cancelled"));
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", ui::error(&format!("Failed to fetch suggestions: {}", e)));
                return Ok(());
            }
        }
    } else if selection == Some(1) {
        // Enter Manually
        let name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Repository Name")
            .interact_text()?;

        let url: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Repository URL (Server line)")
            .interact_text()?;
        (name, url)
    } else {
        println!("{}", ui::warning("Cancelled"));
        return Ok(());
    };
    
    // Optional: SigLevel
    let siglevel: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("SigLevel (Optional, press Enter for default)")
        .allow_empty(true)
        .interact_text()?;

    println!("\nAbout to add the following repository to /etc/pacman.conf:");
    println!("[{}]", name);
    println!("Server = {}", url);
    if !siglevel.is_empty() {
        println!("SigLevel = {}", siglevel);
    }

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed?")
        .default(true)
        .interact()?
    {
        crate::pacman::repos::add_repo(&name, &url, if siglevel.is_empty() { None } else { Some(&siglevel) })?;
        println!("{}", ui::success("Repository added successfully"));
    } else {
        println!("{}", ui::warning("Operation cancelled"));
    }

    Ok(())
}


async fn add_flatpak_remote() -> Result<()> {
    if !crate::flatpak::is_available() {
        println!("{}", ui::error("Flatpak is not installed"));
        return Ok(());
    }

    let methods = vec!["Browse Suggested Remotes".to_string(), "Enter Manually".to_string()];
    let selection = crate::cli::selector::select_string("How do you want to add the remote?", &methods, true)?;

    let (name, url) = if selection == Some(0) {
        // Browse Suggested
        println!("{}", ui::info("Fetching suggested remotes..."));
        match crate::flatpak::remotes::fetch_suggested_remotes().await {
            Ok(suggestions) => {
                let suggestions: Vec<crate::flatpak::remotes::SuggestedRemote> = suggestions;
                if suggestions.is_empty() {
                    println!("{}", ui::warning("No suggestions found"));
                    return Ok(());
                }

                let items: Vec<crate::cli::selector::SelectItem> = suggestions.iter()
                    .map(|r| crate::cli::selector::SelectItem::with_desc(
                        format!("{} - {}", r.name, r.title),
                        &r.url
                    ))
                    .collect();

                match crate::cli::selector::select_items("Select a remote", None, &items)? {
                    Some(idx) => {
                        let selected = &suggestions[idx];
                        (selected.name.clone(), selected.url.clone())
                    }
                    None => {
                        println!("{}", ui::warning("Cancelled"));
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", ui::error(&format!("Failed to fetch suggestions: {}", e)));
                return Ok(());
            }
        }
    } else if selection == Some(1) {
        // Enter Manually
        let name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Remote Name (e.g., flathub)")
            .interact_text()?;

        let url: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Remote URL")
            .interact_text()?;
        (name, url)
    } else {
        println!("{}", ui::warning("Cancelled"));
        return Ok(());
    };

    println!("\nAbout to add remote:");
    println!("Name: {}", name.as_str().bold());
    println!("URL:  {}", url.as_str().dimmed());

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Proceed?")
        .default(true)
        .interact()?
    {
        crate::flatpak::remotes::add_remote(&name, &url)?;
        println!("{}", ui::success("Remote added successfully"));
    } else {
        println!("{}", ui::warning("Operation cancelled"));
    }

    Ok(())
}

async fn remove_repo() -> Result<()> {
    println!("{}", ui::section_header("Remove Repository"));

    let types = vec!["Pacman (Arch Linux)", "Flatpak Remote"];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select repository type")
        .default(0)
        .items(&types)
        .interact()?;

    match selection {
        0 => remove_pacman_repo().await,
        1 => remove_flatpak_remote().await,
        _ => Ok(()),
    }
}

async fn remove_pacman_repo() -> Result<()> {
    // List available repos to select from
    let repos = crate::pacman::repos::list_repos()?;
    if repos.is_empty() {
        println!("{}", ui::warning("No custom repositories found to remove"));
        return Ok(());
    }

    let items: Vec<String> = repos.iter().map(|r| format!("{}: {}", r.name, r.url)).collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select repository to remove")
        .items(&items)
        .interact()?;

    let selected_repo = &repos[selection];

    println!("{}", ui::warning(&format!("About to remove repository '{}' from /etc/pacman.conf", selected_repo.name)));
    println!("Note: This will try to comment out the section.");

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Are you sure?")
        .default(false)
        .interact()?
    {
        crate::pacman::repos::remove_repo(&selected_repo.name)?;
        println!("{}", ui::success("Repository removed successfully"));
    } else {
        println!("{}", ui::warning("Operation cancelled"));
    }

    Ok(())
}

async fn remove_flatpak_remote() -> Result<()> {
    let remotes = crate::flatpak::remotes::list_remotes()?;
    if remotes.is_empty() {
        println!("{}", ui::warning("No remotes found"));
        return Ok(());
    }

    let items: Vec<String> = remotes.iter().map(|r| format!("{}: {}", r.name, r.url)).collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select remote to remove")
        .items(&items)
        .interact()?;

    let selected_remote = &remotes[selection];

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(&format!("Remove remote '{}'?", selected_remote.name))
        .default(false)
        .interact()?
    {
        crate::flatpak::remotes::remove_remote(&selected_remote.name)?;
        println!("{}", ui::success("Remote removed successfully"));
    } else {
        println!("{}", ui::warning("Operation cancelled"));
    }

    Ok(())
}
