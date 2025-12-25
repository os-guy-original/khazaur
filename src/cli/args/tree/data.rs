use crate::error::Result;
use std::process::Command;



pub fn get_flat_tree(package: &str) -> Result<Vec<(usize, String)>> {
    if Command::new("which").arg("pactree").output().map(|o| o.status.success()).unwrap_or(false) {
        let output = Command::new("pactree")
            //.arg("-u") 
            .arg(package)
            .output()?;
            
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut result = Vec::new();
            
            for line in stdout.lines() {
                 // Counts logic
                 // "──" match count?
                 // pactree uses "└─" or "├─"
                 let depth = line.matches("─").count();
                 let clean_name = line.trim_start_matches(|c| c == ' ' || c == '│' || c == '├' || c == '└' || c == '─').trim();
                 result.push((depth, clean_name.to_string()));
            }
            return Ok(result);
        }
    }
    
    // Fallback
    Ok(vec![(0, package.to_string()), (1, "Dependencies not available (pactree missing)".to_string())])
}
