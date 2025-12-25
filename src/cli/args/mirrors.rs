use crate::error::{KhazaurError, Result};
use crate::ui;
use std::process::{Command, Stdio};
use std::io::Write;

pub fn update_mirrors(country: Option<String>, fast: bool) -> Result<()> {
    println!("{}", ui::section_header("Updating Mirrorlist"));

    // Check for reflector
    if Command::new("which").arg("reflector").output().map(|o| o.status.success()).unwrap_or(false) {
        println!("{}", ui::info("Using 'reflector' to find fastest mirrors..."));
        
        let mut cmd = Command::new("reflector"); // No sudo yet, just fetching
        
        if let Some(c) = country {
            cmd.arg("--country").arg(c);
        } else {
             cmd.arg("--latest").arg("20");
        }
        
        if fast {
            cmd.arg("--sort").arg("rate");
        } else {
            cmd.arg("--sort").arg("age");
        }
        
        cmd.arg("--protocol").arg("https");
        cmd.arg("--number").arg("10"); // Top 10
        // No save arg, output to stdout
        
        println!("{}", ui::info("Ranking mirrors (please wait)..."));
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            return Err(KhazaurError::Config("Reflector failed to fetch mirrors".into()).into());
        }
        
        let mirrors = String::from_utf8_lossy(&output.stdout);
        
        if mirrors.trim().is_empty() {
            return Err(KhazaurError::Config("No mirrors found".into()).into());
        }

        println!("\n{}", ui::section_header("Top Mirrors Found"));
        // Display a preview (first 5 lines or so)
        for (i, line) in mirrors.lines().filter(|l| l.starts_with("Server")).take(5).enumerate() {
            println!(" {}. {}", i+1, line.replace("Server = ", "").trim());
        }
        if mirrors.lines().filter(|l| l.starts_with("Server")).count() > 5 {
            println!(" ... and more");
        }
        
        println!("\nDo you want to write these to /etc/pacman.d/mirrorlist? [y/N]");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") {
            println!("{}", ui::info("Writing to mirrorlist (sudo required)..."));
            
            let mut tee = Command::new("sudo")
                .arg("tee")
                .arg("/etc/pacman.d/mirrorlist")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?;
                
            if let Some(mut stdin) = tee.stdin.take() {
                stdin.write_all(mirrors.as_bytes())?;
            }
            
            let status = tee.wait()?;
            
            if status.success() {
                println!("{}", ui::success("Mirrorlist updated successfully"));
            } else {
                return Err(KhazaurError::Config("Failed to write mirrorlist".into()).into());
            }
        } else {
            println!("{}", ui::warning("Operation cancelled. Mirrorlist unchanged."));
        }
        
    } else {
        println!("{}", ui::warning("'reflector' not found."));
        println!("{}", ui::info("Basic fetch cannot verify speed. Please install 'reflector' for ranking."));
        println!("Install now? [y/N]");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") {
            // Call install logic or just generic warning?
            // Calling generic install might be recursively complex if we are inside an update loop, but it's fine.
            // But we can't easily access `Args::execute` here. 
            // Better to tell user to install it.
            println!("Please run: khazaur -S reflector");
        }
        
        // Keep fallback logic? User asked for "find fastest". Fallback doesn't do that.
        // If they decline install, maybe we just show current logic?
        // Let's keep old behavior for fallback but warn.
        // Actually, let's just return if they don't have reflector if the specific goal is "find fastest".
        
        println!("{}", ui::info("Falling back to fetching standard list..."));
        
        let url = "https://archlinux.org/mirrorlist/?country=all&protocol=https&ip_version=4";
        let output = Command::new("curl").arg("-s").arg(url).output()?;
        
        if !output.status.success() {
             return Err(KhazaurError::Config("Failed to fetch mirrorlist".into()).into());
        }
        
        let raw_list = String::from_utf8_lossy(&output.stdout);
        let clean_list = raw_list.replace("#Server", "Server");
        
        println!("\nFetched list (unranked). First few entries:");
        for line in clean_list.lines().filter(|l| l.starts_with("Server")).take(3) {
             println!(" - {}", line.replace("Server = ", "").trim());
        }
        
        println!("\nWrite to /etc/pacman.d/mirrorlist? [y/N]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim().eq_ignore_ascii_case("y") {
             let mut tee = Command::new("sudo")
                .arg("tee")
                .arg("/etc/pacman.d/mirrorlist")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()?;
            if let Some(mut stdin) = tee.stdin.take() {
                stdin.write_all(clean_list.as_bytes())?;
            }
            if tee.wait()?.success() {
                println!("{}", ui::success("Mirrorlist updated (unranked)"));
            }
        } else {
             println!("Cancelled.");
        }
    }
    
    Ok(())
}
