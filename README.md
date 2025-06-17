# Rust Distributed Key-Value Store

## Project Structure

```
distributed-key-value-store/
├── src/
│   ├── main.rs
│   ├── config.rs              # CLI args and config parsing
│   ├── cluster/               # Cluster logic: leader election, peer health, quorum
│   │   ├── mod.rs
│   │   ├── peer.rs            # Peer representation & communication
│   │   ├── election.rs        # Static-priority leader election
│   │   └── quorum.rs          # Quorum read/write logic
│   ├── store/                 # Key-value storage + persistence
│   │   ├── mod.rs
│   │   ├── engine.rs          # In-memory or sled-backed key-value store
│   │   ├── lamport.rs         # Lamport clock implementation
│   │   ├── wal.rs             # WAL logging & replay
│   │   └── snapshot.rs        # Compaction + snapshot handling
│   ├── replication/           # Batch replication system
│   │   ├── mod.rs
│   │   ├── batch.rs           # Queue, flush interval
│   │   └── handler.rs         # Handles /replicate-batch endpoint
│   ├── api/                   # HTTP routes and handlers
│   │   ├── mod.rs
│   │   ├── client.rs          # PUT/GET/DELETE routes
│   │   ├── internal.rs        # /internal/* endpoints like /replicate, /ping
│   │   └── metrics.rs         # Prometheus metrics export
│   ├── cli/                   # CLI client implementation
│   │   ├── mod.rs
│   │   └── client.rs          # Interacts with the cluster (put/get/delete)
│   ├── util/                  # Common types & helpers
│   │   ├── mod.rs
│   │   └── types.rs           # Shared types: Value, Request, NodeID, etc.
│   └── logger.rs              # Logging setup (e.g., tracing or log crate)
├── Cargo.toml
├── README.md
└── scripts/
    ├── start_cluster.sh       # Spins up 3+ nodes
    └── test_put_get.sh        # CLI test driver
```