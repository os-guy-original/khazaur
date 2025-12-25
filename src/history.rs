use crate::error::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub action: String,
    pub packages: Vec<String>,
    pub success: bool,
}

pub fn log_action(action: &str, packages: &[String], success: bool) -> Result<()> {
    let entry = HistoryEntry {
        timestamp: Local::now().to_rfc3339(),
        action: action.to_string(),
        packages: packages.to_vec(),
        success,
    };

    let log_path = get_history_path()?;
    
    // Ensure directory exists
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let json = serde_json::to_string(&entry)?;
    writeln!(file, "{}", json)?;

    Ok(())
}

pub fn get_history(limit: usize) -> Result<Vec<HistoryEntry>> {
    let log_path = get_history_path()?;
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(log_path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        if let Ok(l) = line {
            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&l) {
                entries.push(entry);
            }
        }
    }

    // Return last 'limit' entries
    Ok(entries.into_iter().rev().take(limit).collect())
}

fn get_history_path() -> Result<PathBuf> {
    // Use data_local_dir for persistent history storage (~/.local/share/khazaur/history.jsonl)
    
    let mut path = dirs::data_local_dir().ok_or(crate::error::KhazaurError::Config("Could not determine data directory".into()))?;
    path.push("khazaur");
    path.push("history.jsonl");
    Ok(path)
}
