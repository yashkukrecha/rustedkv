use crate::util::{LogEntry, Operation};
use std::{fs::File, io::{BufReader, BufRead}, path::Path};

pub fn replay_wal(path: &str) -> anyhow::Result<Vec<LogEntry>> {
    let file = File::open(Path::new(path))?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let entry: LogEntry = serde_json::from_str(&line)?;
        entries.push(entry);
    }

    Ok(entries)
}