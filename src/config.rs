use clap::Parser;

#[derive(Parser, Debug)]
pub struct CliArgs {
    #[arg(long)]
    pub node_id: u64,

    #[arg(long)]
    pub address: String,

    #[arg(long)]
    pub leader_id: u64,

    #[arg(long, value_delimiter = ',')]
    pub peer_addresses: Vec<String>
}