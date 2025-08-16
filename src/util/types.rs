use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Put { key: String, value: String },
    Delete { key: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts: u64,
    pub node_id: u64, // used for tie-breaking in Lamport clocks
    pub operation: Operation,
}