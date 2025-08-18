use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, mpsc};
use crate::store::{Store, LamportClock, Wal};
use crate::cluster::{ClusterState};
use crate::api::{Metrics};
use crate::util::{LogEntry};

#[derive(Clone, Debug)]
pub struct ApiState {
    pub store: Arc<Store>,
    pub clock: Arc<LamportClock>,
    pub cluster: Arc<RwLock<ClusterState>>,
    pub metrics: Metrics,
    pub wal: Arc<Mutex<Wal>>,
    pub rep_tx: mpsc::Sender<LogEntry>,

    pub chaos_before_sync_ms: u64, // testing chaos injection
}
