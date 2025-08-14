use crate::util::{LogEntry, Operation};
use crate::store::{Store, Value, replay_wal};

// TODO: IMPLEMENT SNAPSHOTTING

pub async fn recover_from_snapshot_and_wal(
    store: &mut Store,
    snapshot_path: &str,
    wal_path: &str,
) -> anyhow::Result<()> {
    let entries = replay_wal(wal_path)?;
    for entry in entries {
        println!("{:?}", entry);
        match entry.operation {
            Operation::Put { key, value } => {
                store.put(key, value, entry.timestamp).await;
            }
            Operation::Delete { key } => {
                store.delete(&key).await;
            }
        }
    }

    Ok(())
}