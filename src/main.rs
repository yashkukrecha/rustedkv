mod api;
mod cluster;
mod store;
mod config;
mod util;

use std::sync::Arc;
use tokio::sync::RwLock;
use clap::Parser;

use crate::api::ApiState;
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
    };

    // TODO: Router::with_state(state.clone()) and spawn background tasks
    // axum::Server::bind(&addr).serve(app.into_make_service()).await?;

    let c = cluster.read().await;
    println!("{:#?}", *c);

    Ok(())
}

