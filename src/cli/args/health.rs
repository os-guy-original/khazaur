use crate::ui;
use crate::error::Result;
use std::process::Command;
use colored::Colorize;
use std::path::Path;

pub fn check_health() -> Result<()> {
    println!("{}", ui::section_header("System Health Check"));
    
    let mut specific_issues = 0;
    
    // 1. Failed Systemd Services
    println!("{}", ui::info("Checking systemd services..."));
    match Command::new("systemctl").args(["--failed", "--no-pager"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let failed_count = stdout.lines().filter(|l| l.contains("loaded units listed")).count(); // Basic heuristic, or parse lines
            // Better: '0 loaded units listed' means clean.
            
            if stdout.contains("0 loaded units listed") {
                 println!("  {}", "✓ No failed services found".green());
            } else {
                 println!("  {}", "✗ Failed systemd services detected:".red());
                 for line in stdout.lines() {
                     if line.contains("●") { // Failed units often marked with bullet
                         println!("    {}", line.trim());
                         specific_issues += 1;
                     }
                 }
            }
        },
        Err(_) => println!("  {}", "? Could not check systemd services".yellow()),
    }
    
    // 2. Pacnew files
    println!("\n{}", ui::info("Checking for .pacnew files..."));
    // Safe way: find /etc -name "*.pacnew" 2>/dev/null
    match Command::new("sudo").args(["find", "/etc", "-name", "*.pacnew"]).output() {
        Ok(output) => {
             let stdout = String::from_utf8_lossy(&output.stdout);
             let files: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
             
             if files.is_empty() {
                 println!("  {}", "✓ No .pacnew files found".green());
             } else {
                 println!("  {}", format!("✗ Found {} .pacnew file(s):", files.len()).red());
                 for f in files {
                     println!("    {}", f);
                 }
                 println!("    {}", "(Merge these files to keep your configuration up to date)".dimmed());
                 specific_issues += 1;
             }
        },
        Err(_) => println!("  {}", "? Could not scan /etc for .pacnew files".yellow()),
    }
    
    // 3. Disk Usage
    println!("\n{}", ui::info("Checking disk space..."));
    match Command::new("df").args(["-h", "/", "/home"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Just print the lines, users can interpret "Use%"
            for line in stdout.lines().skip(1) { // Skip header if repeated or just let it be
                println!("  {}", line);
                // Heuristic: check if Use% > 90%
                if let Some(pos) = line.find('%') {
                     // Parse number before %
                     // This is brittle parsing, but helpful warning
                     // e.g. " /dev/sda1 ... 12G 95% /"
                     // Quick & dirty check:
                     let parts: Vec<&str> = line.split_whitespace().collect();
                     for part in parts {
                         if part.ends_with('%') {
                             if let Ok(pct) = part.replace('%', "").parse::<u8>() {
                                 if pct > 90 {
                                     println!("    {}", format!("! Warning: High disk usage detected on volume ({})", pct).red());
                                     specific_issues += 1;
                                 }
                             }
                         }
                     }
                }
            }
        },
        Err(_) => println!("  {}", "? Could not check disk usage".yellow()),
    }
    
    // 4. Stale Locks
    println!("\n{}", ui::info("Checking for stale lock files..."));
    let lock_file = Path::new("/var/lib/pacman/db.lck");
    if lock_file.exists() {
        println!("  {}", format!("✗ Pacman lock file found at {:?}", lock_file).red());
        println!("    {}", "(If pacman is not running, remove this file to fix updates)".dimmed());
        specific_issues += 1;
    } else {
        println!("  {}", "✓ No stale pacman lock file found".green());
    }
    
    println!("\n{}", ui::section_header("Health Check Complete"));
    if specific_issues == 0 {
        println!("{}", ui::success("System looks healthy! 🚀"));
    } else {
        println!("{}", ui::warning(&format!("Found {} potential issue(s) to address.", specific_issues)));
    }
    
    Ok(())
}
