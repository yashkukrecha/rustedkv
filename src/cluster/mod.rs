use std::collections::HashMap;
use std::sync::RwLock;
use crate::CliArgs;

pub struct ClusterState {
    pub node_id: u64,
    pub address: String,
    pub leader_id: u64,
    pub peer_addresses: Vec<String>,
    pub is_alive: HashMap<u64, bool>
}

impl ClusterState {
    pub fn from_args(args: CliArgs) -> Self {
        ClusterState {
            node_id: args.node_id,
            address: args.address,
            leader_id: args.leader_id,
            peer_addresses: args.peer_addresses,
            is_alive: HashMap::new()
        }
    }
}