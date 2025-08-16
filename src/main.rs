mod api;
mod cluster;
mod store;
mod config;
mod util;

use std::{env, sync::Arc, net::SocketAddr};
use axum::serve;
use tokio::{sync::{RwLock, Mutex}, net::TcpListener};
use clap::Parser;

use crate::api::{ApiState, Metrics, RouterBuilder};
use crate::cluster::ClusterState;
use crate::config::CliArgs;
use crate::store::{Store, LamportClock, Wal, recover_from_snapshot_and_wal};

// Testing chaos configuration
#[derive(Clone, Debug)]
struct ChaosCfg {
    /// If >0, sleep this many milliseconds AFTER WAL append and BEFORE fsync.
    /// Lets you kill the process and produce a torn final record.
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
    let chaos = ChaosCfg::from_env();

    // Assemble API state
    let state = ApiState { 
        store: Arc::clone(&store), 
        clock: Arc::clone(&clock), 
        cluster: Arc::clone(&cluster),
        metrics: Metrics::new(),
        wal: Arc::clone(&wal),
        chaos_before_sync_ms: chaos.before_sync_ms,
    };

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
        c.address.parse()?
    };
    let app = RouterBuilder::with_state(state);

    println!("Listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    serve(listener, app).await?;

    Ok(())
}

