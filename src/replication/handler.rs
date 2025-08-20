use std::time::Duration;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use reqwest::Client;
use serde::{Serialize, Deserialize};

use crate::cluster::ClusterState;
use crate::util::LogEntry;

const BATCH_MAX: usize = 128;
const FLUSH_MS: u64 = 500;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplicateBody {
    pub entries: Vec<LogEntry>
}

// creates a channel for each peer and broadcasts new LogEntries to all peers
pub fn spawn_leader_replicator(
    cluster: Arc<RwLock<ClusterState>>,
    mut rx: mpsc::Receiver<LogEntry>,
) -> tokio::task::JoinHandle<()> {
    
    tokio::spawn(async move {
        let client = Client::new();
        let peers = cluster.read().await.peer_addresses.clone();

        let mut peer_txs = Vec::new();
        for peer in peers {
            let (tx, rx_peer) = mpsc::channel::<LogEntry>(1024);
            peer_txs.push(tx);
            tokio::spawn(peer_worker(client.clone(), Arc::clone(&cluster), peer, rx_peer));
        }

        while let Some(entry) = rx.recv().await {
            for tx in &peer_txs {
                let _ = tx.send(entry.clone()).await;
            }
        }
    })
}

// thread for a peer that flushes whenever the batch is full or every FLUSH_MS
async fn peer_worker(client: Client, cluster: Arc<RwLock<ClusterState>>, peer_base: String, mut rx: mpsc::Receiver<LogEntry>) {
    let url = format!("{}/replicate", peer_base.trim_end_matches('/'));
    let mut batch = Vec::with_capacity(BATCH_MAX);
    let mut interval = tokio::time::interval(Duration::from_millis(FLUSH_MS));

    loop {
        tokio::select! {
            maybe = rx.recv() => {
                match maybe {
                    None => break,
                    Some(entry) => {
                        batch.push(entry);
                        if batch.len() >= BATCH_MAX {
                            let is_alive = {
                                let c = cluster.read().await;
                                *c.is_alive.get(&peer_base).unwrap_or(&false)
                            };
                            if is_alive {
                                flush(&client, &url, &mut batch).await;
                            }
                        }
                    }
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    let is_alive = {
                        let c = cluster.read().await;
                        *c.is_alive.get(&peer_base).unwrap_or(&false)
                    };
                    if is_alive {
                        flush(&client, &url, &mut batch).await;
                    }
                }
            }
        }
    }
}

// sends the batch to the peer and clears it on success
async fn flush(client: &Client, url: &str, batch: &mut Vec<LogEntry>) {
    if batch.is_empty() { return; }
    let body = ReplicateBody { entries: batch.clone() };
    let resp = client.post(url)
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) => {
            if !r.status().is_success() {
                eprintln!("Replication to {} failed with status: {}", url, r.status());
            } else {
                batch.clear();
            }
        }
        Err(e) => {
            eprintln!("Replication to {} failed: {}", url, e);
        }
    }
}