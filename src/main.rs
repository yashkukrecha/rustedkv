mod api;
mod cluster;
mod store;
mod config;
mod util;
mod replication;

use std::{collections::HashMap, time::Duration, env, sync::Arc, net::SocketAddr};
use axum::serve;
use tokio::{time::{interval, MissedTickBehavior}, sync::{mpsc, RwLock, Mutex}, net::TcpListener};
use clap::Parser;
use reqwest::Client;

use crate::api::{ApiState, Metrics, RouterBuilder};
use crate::cluster::ClusterState;
use crate::config::CliArgs;
use crate::store::{Store, LamportClock, Wal, recover_from_snapshot_and_wal};
use crate::replication::{spawn_leader_replicator};

// Testing chaos configuration
#[derive(Clone, Debug)]
struct ChaosCfg {
    // If > 0, sleep this many milliseconds AFTER WAL append and BEFORE fsync.
    // Lets you kill the process and produce a torn final record.
    before_sync_ms: u64,
}

impl ChaosCfg {
    fn from_env() -> Self {
        let val = env::var("CHAOS_BEFORE_SYNC_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        Self { before_sync_ms: val }
    }
}

fn spawn_heartbeat(cluster: Arc<RwLock<ClusterState>>) {
    println!("Spawning heartbeat task...");
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client");

    tokio::spawn(async move {
        println!("Heartbeat task started");
        let mut ticker = interval(Duration::from_secs(5));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;
            let c = cluster.read().await;
            let address = c.address.clone();
            let peers = c.peer_addresses.clone();
            drop(c);

            let mut new_alive = HashMap::new();
            new_alive.insert(address.clone(), true);
            for peer in peers {
                let url = format!("{}/ping", peer.trim_end_matches('/'));
                println!("Pinging {}", url);
                let is_up = client.get(&url).send().await.is_ok();
                new_alive.insert(peer, is_up);
            }

            let mut c = cluster.write().await;
            c.is_alive = new_alive;
            drop(c);
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Assemble core state
    let mut store = Store::new();
    let clock = LamportClock::new();
    let cluster = Arc::new(RwLock::new(ClusterState::from(args)));

    // Recover BEFORE wrapping in Arc
    let snapshot_path = "snapshots/latest.snap"; // UNUSED
    let wal_path = std::env::var("WAL_PATH").unwrap_or_else(|_| "wal.log".to_string());
    recover_from_snapshot_and_wal(&mut store, &clock, snapshot_path, &wal_path).await?;

    println!("Recovered store state: {:#?}", store);

    // Wrap recovered store + open WAL for runtime appends
    let store = Arc::new(store);
    let clock = Arc::new(clock);
    let wal = Arc::new(Mutex::new(Wal::open(&wal_path)?));
    let (rep_tx, rep_rx) = mpsc::channel::<util::LogEntry>(4096);
    let chaos = ChaosCfg::from_env();

    let leader_id = { cluster.read().await.leader_id };
    let node_id = { cluster.read().await.node_id };

    if node_id == leader_id {
        spawn_leader_replicator(Arc::clone(&cluster), rep_rx);
    } else {
        drop(rep_rx); // non-leaders don't replicate
    }

    // Assemble API state
    let state = ApiState { 
        store: Arc::clone(&store), 
        clock: Arc::clone(&clock), 
        cluster: Arc::clone(&cluster),
        metrics: Metrics::new(),
        wal: Arc::clone(&wal),
        rep_tx: rep_tx,
        chaos_before_sync_ms: chaos.before_sync_ms,
    };

    // Heartbeat
    spawn_heartbeat(Arc::clone(&cluster));

    // Startup
    let c = cluster.read().await;
    println!(
        "Node starting: node_id={}, listen_addr={}, wal_path={}, chaos_before_sync_ms={}",
        c.node_id, c.address, wal_path, chaos.before_sync_ms
    );
    drop(c);

    // ---- Serve ----
    let addr: SocketAddr = {
        // Prefer ClusterState’s addr if you already store it there
        let c = cluster.read().await;
        let addr_str = c.address.strip_prefix("http://").unwrap_or(&c.address);
        addr_str.parse()?
    };
    let app = RouterBuilder::with_state(state);

    println!("Listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    serve(listener, app).await?;

    Ok(())
}

