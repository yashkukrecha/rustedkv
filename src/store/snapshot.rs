use crate::util::{LogEntry, Operation};
use crate::store::{Store, Value, LamportClock, replay_wal};

// TODO: IMPLEMENT SNAPSHOTTING

pub async fn recover_from_snapshot_and_wal(
    store: &mut Store,
    clock: &LamportClock,
    snapshot_path: &str,
    wal_path: &str,
) -> anyhow::Result<()> {
    let entries = replay_wal(wal_path)?;
    for entry in entries {
        println!("{:?}", entry);
        clock.tick_observe(entry.ts);
        match entry.operation {
            Operation::Put { key, value } => {
                store.put(key, value, entry.ts, entry.node_id).await;
            }
            Operation::Delete { key } => {
                store.delete(&key, entry.ts, entry.node_id).await;
            }
        }
    }

    Ok(())
}