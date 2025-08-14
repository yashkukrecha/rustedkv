mod api;
mod cluster;
mod store;
mod config;
mod util;

use std::{sync::Arc, net::SocketAddr};
use axum::serve;
use tokio::{sync::{RwLock, Mutex}, net::TcpListener};
use clap::Parser;

use crate::api::{ApiState, Metrics, RouterBuilder};
use crate::cluster::ClusterState;
use crate::config::CliArgs;
use crate::store::{Store, LamportClock, Wal, recover_from_snapshot_and_wal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Assemble core state
    let mut store = Store::new();
    let clock = Arc::new(LamportClock::new(0));
    let cluster = Arc::new(RwLock::new(ClusterState::from(args)));

    // Recover BEFORE wrapping in Arc
    let snapshot_path = "snapshots/latest.snap"; // UNUSED
    let wal_path = "wal.log";
    recover_from_snapshot_and_wal(&mut store, snapshot_path, wal_path).await?;

    println!("Recovered store state: {:#?}", store);

    // Wrap recovered store + open WAL for runtime appends
    let store = Arc::new(store);
    let wal = Arc::new(Mutex::new(Wal::open(wal_path)?));

    // Assemble API state
    let state = ApiState { 
        store: Arc::clone(&store), 
        clock: Arc::clone(&clock), 
        cluster: Arc::clone(&cluster),
        metrics: Metrics::new(),
        wal: Arc::clone(&wal),
    };

    // TODO: Router::with_state(state.clone()) and spawn background tasks
    // axum::Server::bind(&addr).serve(app.into_make_service()).await?;

    let c = cluster.read().await;
    println!("{:#?}", *c);

    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let app = RouterBuilder::with_state(state);

    println!("Listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    serve(listener, app).await?;

    Ok(())
}

