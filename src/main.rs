pub mod config;
pub mod cluster;

use clap::Parser;
use std::any::type_name;
use crate::config::CliArgs;
use crate::cluster::ClusterState;

fn type_of<T>(_: &T) -> &str {
    std::any::type_name::<T>()
}

fn main() {
    let args = CliArgs::parse();
    let cluster_state = ClusterState::from_args(args);
}
