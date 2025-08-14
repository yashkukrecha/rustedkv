mod api;
mod cluster;
mod store;
mod config;
mod util;

use std::{sync::Arc, net::SocketAddr};
use axum::serve;
use tokio::{sync::RwLock, net::TcpListener};
use clap::Parser;

use crate::api::{ApiState, Metrics, RouterBuilder};
use crate::cluster::ClusterState;
use crate::config::CliArgs;
use crate::store::engine::Store;
use crate::store::lamport::LamportClock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    // Build core state
    let store = Arc::new(Store::new());
    let clock = Arc::new(LamportClock::new(0));
    let cluster = Arc::new(RwLock::new(ClusterState::from(args)));

    // Assemble API state
    let state = ApiState { 
        store: Arc::clone(&store), 
        clock: Arc::clone(&clock), 
        cluster: Arc::clone(&cluster),
        metrics: Metrics::new(),
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

