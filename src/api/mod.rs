pub mod client;
pub mod internal;
pub mod metrics;

use std::sync::{Arc, RwLock, Mutex};
use crate::store::engine::Store;
use crate::cluster::ClusterState;
use crate::store::LamportClock;

#[derive(Clone, Debug)]
pub struct ApiState {
    pub store: Arc<Store>,
    pub clock: Arc<LamportClock>,
    pub cluster: Arc<RwLock<ClusterState>>
}