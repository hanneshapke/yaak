use chrono::{DateTime, Local};
use colored::Colorize;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Local>,
    pub description: String,
    pub command: String,
    pub model: String,
}

/// Returns the path to the history file (~/.local/share/yaak/history.jsonl).
pub fn history_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join("yaak").join("history.jsonl")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".local")
            .join("share")
            .join("yaak")
            .join("history.jsonl")
    } else {
        PathBuf::from("history.jsonl")
    }
}

/// Append a history entry to the JSONL file.
pub fn save_entry(description: &str, command: &str, model: &str) {
    let entry = HistoryEntry {
        timestamp: Local::now(),
        description: description.to_string(),
        command: command.to_string(),
        model: model.to_string(),
    };

    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let line = match serde_json::to_string(&entry) {
        Ok(json) => json,
        Err(_) => return,
    };

    let mut file = match fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let _ = writeln!(file, "{}", line);
}

/// Load all history entries from the JSONL file.
pub fn load_entries() -> Vec<HistoryEntry> {
    let path = history_path();
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

/// Display the N most recent history entries.
pub fn show_history(limit: usize) {
    let entries = load_entries();
    if entries.is_empty() {
        eprintln!("{}", t!("history_empty").dimmed());
        return;
    }

    let start = entries.len().saturating_sub(limit);
    let recent = &entries[start..];

    eprintln!("{}", "─".repeat(60).dimmed());
    eprintln!("  {}", t!("history_title").bold());
    eprintln!("{}", "─".repeat(60).dimmed());

    for (i, entry) in recent.iter().enumerate() {
        let idx = start + i + 1;
        let time = entry.timestamp.format("%Y-%m-%d %H:%M");
        eprintln!(
            "  {} {}  {}",
            format!("{:>4}.", idx).dimmed(),
            time.to_string().dimmed(),
            entry.command.green()
        );
        eprintln!("        {} {}", "⟩".dimmed(), entry.description.dimmed());
    }
    eprintln!("{}", "─".repeat(60).dimmed());
    eprintln!(
        "  {}",
        t!("history_entries_total", total = entries.len(), shown = recent.len())
    );
}

/// Print the most recent command to stderr and return it for execution.
pub fn get_last() -> Option<HistoryEntry> {
    let entries = load_entries();
    entries.into_iter().last()
}

/// Search history entries by substring match on description or command.
pub fn search_history(query: &str) {
    let entries = load_entries();
    let query_lower = query.to_lowercase();

    let matches: Vec<(usize, &HistoryEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e.description.to_lowercase().contains(&query_lower)
                || e.command.to_lowercase().contains(&query_lower)
        })
        .collect();

    if matches.is_empty() {
        eprintln!(
            "{} {}",
            t!("info_prefix").dimmed(),
            t!("history_no_matches", query = query)
        );
        return;
    }

    eprintln!("{}", "─".repeat(60).dimmed());
    eprintln!("  {} \"{}\"", t!("history_search_results").bold(), query.bold());
    eprintln!("{}", "─".repeat(60).dimmed());

    for (idx, entry) in &matches {
        let time = entry.timestamp.format("%Y-%m-%d %H:%M");
        eprintln!(
            "  {} {}  {}",
            format!("{:>4}.", idx + 1).dimmed(),
            time.to_string().dimmed(),
            entry.command.green()
        );
        eprintln!("        {} {}", "⟩".dimmed(), entry.description.dimmed());
    }
    eprintln!("{}", "─".repeat(60).dimmed());
    eprintln!("  {}", t!("history_matches", count = matches.len()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn sample_entry() -> HistoryEntry {
        HistoryEntry {
            timestamp: Local::now(),
            description: "list files".to_string(),
            command: "ls -la".to_string(),
            model: "gpt-4o-mini".to_string(),
        }
    }

    #[test]
    fn entry_serializes_to_json() {
        let entry = sample_entry();
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"command\":\"ls -la\""));
        assert!(json.contains("\"description\":\"list files\""));
    }

    #[test]
    fn entry_roundtrips_through_json() {
        let entry = sample_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.description, "list files");
        assert_eq!(parsed.model, "gpt-4o-mini");
    }

    #[test]
    fn load_entries_handles_missing_file() {
        // load_entries should return empty vec for non-existent file
        let entries = load_entries();
        // This test just verifies it doesn't panic; actual result depends on env
        let _ = entries;
    }

    #[test]
    fn load_entries_parses_jsonl() {
        let entry1 = sample_entry();
        let entry2 = HistoryEntry {
            description: "find rust files".to_string(),
            command: "find . -name '*.rs'".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            ..sample_entry()
        };

        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "{}", serde_json::to_string(&entry1).unwrap()).unwrap();
        writeln!(tmpfile, "{}", serde_json::to_string(&entry2).unwrap()).unwrap();

        let file = fs::File::open(tmpfile.path()).unwrap();
        let entries: Vec<HistoryEntry> = BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| serde_json::from_str(&l).ok())
            .collect();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "ls -la");
        assert_eq!(entries[1].command, "find . -name '*.rs'");
    }
}
