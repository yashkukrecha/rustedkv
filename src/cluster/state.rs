use std::collections::HashMap;
use crate::config::CliArgs;

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub node_id: u64,
    pub address: String,
    pub leader_id: u64,
    pub peer_addresses: Vec<String>,
    pub is_alive: HashMap<String, bool>
}

impl From<CliArgs> for ClusterState {
    fn from(args: CliArgs) -> Self {

        let peers = args.peer_addresses;
        let mut is_alive = HashMap::new();
        for peer in &peers {
            is_alive.insert(peer.to_string(), false);
        }
        let address = args.address;
        is_alive.insert(address.clone(), true);

        ClusterState {
            node_id: args.node_id,
            address: address,
            leader_id: args.leader_id,
            peer_addresses: peers,
            is_alive: is_alive,
        }
    }
}