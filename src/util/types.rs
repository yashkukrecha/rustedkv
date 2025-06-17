#[derive(Serialize, Deserialize)]
pub enum Operation {
    Put { key: String, value: String },
    Delete { key: String },
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub operation: Operation,
}