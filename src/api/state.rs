use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use crate::store::{Store, LamportClock, Wal};
use crate::cluster::{ClusterState};
use crate::api::{Metrics};

#[derive(Clone, Debug)]
pub struct ApiState {
    pub store: Arc<Store>,
    pub clock: Arc<LamportClock>,
    pub cluster: Arc<RwLock<ClusterState>>,
    pub metrics: Metrics,
    pub wal: Arc<Mutex<Wal>>,
}
