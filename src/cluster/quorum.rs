use std::time::Duration;
use tokio::time::timeout;
use reqwest::Client;
use futures::stream::{FuturesUnordered, StreamExt};

use crate::util::LogEntry;
use crate::store::{Value};

// Quorum write function that sends log entries to peers and waits for acknowledgments
pub async fn quorum_write(
    log_entry: LogEntry,
    peers: Vec<String>,
    write_quorum: usize,
) -> Result<(), ()> {
    let client = Client::new();
    let mut tasks = FuturesUnordered::new();

    for peer in peers {
        let url = format!("{}/replicate", peer.trim_end_matches('/'));
        let entry = log_entry.clone();
        let client_clone = client.clone();

        tasks.push(tokio::spawn(async move {
            let res = timeout(
                Duration::from_secs(2),
                client_clone
                    .post(&url)
                    .json(&serde_json::json!({"entries": [entry]}))
                    .send(),
            )
            .await;
            if let Ok(Ok(response)) = res {
                response.status().is_success()
            } else {
                false
            }
        }));
    }

    let mut acks = 1;
    if acks >= write_quorum {
        return Ok(());
    }

    while let Some(task) = tasks.next().await {
        if let Ok(true) = task {
            acks += 1;
            if acks >= write_quorum {
                return Ok(());
            }
        }
    }
    Err(())
}

// Quorum read function that sends read requests to peers and waits for responses
pub async fn quorum_read(
    key: String,
    local_value: Value,
    peers: Vec<String>,
    read_quorum: usize,
) -> Result<Value, ()> {
    let client = Client::new();
    let mut responses = Vec::new();

    responses.push(local_value);

    let mut tasks = FuturesUnordered::new();

    for peer in peers {
        let key_clone = key.clone();
        let client_clone = client.clone();
        let url = format!(
            "{}/key/{}",
            peer.trim_end_matches('/'),
            key_clone
        );

        tasks.push(async move {
            match timeout(Duration::from_secs(2), client_clone.get(&url).send()).await {
                Ok(Ok(resp)) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let (Some(ts), Some(node_id)) = (
                            json.get("ts").and_then(|v| v.as_u64()),
                            json.get("node_id").and_then(|v| v.as_u64()),
                        ) {
                            let data = json.get("data").and_then(|v| v.as_str()).map(|s| s.to_string());
                            return Some(Value { data, ts, node_id });
                        }
                    }
                }
                _ => {}
            }
            None
        });
    }

    while let Some(maybe) = tasks.next().await {
        if let Some(val) = maybe {
            responses.push(val);
        }
    }

    if responses.len() < read_quorum {
        return Err(());
    }

    // Select the freshest value by comparing (ts, node_id) where higher is newer
    responses.sort_by_key(|v| (v.ts, v.node_id));
    responses.pop().ok_or(())
}