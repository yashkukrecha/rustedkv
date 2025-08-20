use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Value {
    pub data: Option<String>,
    pub ts: u64,
    pub node_id: u64, // for tie-breaking in Lamport clocks
}

impl Value {
    pub fn is_newer_than(&self, other: Option<&Value>) -> bool {
        match other {
            None => true,
            Some(v) => (self.ts > v.ts) || (self.ts == v.ts && self.node_id > v.node_id),
        }
    }
}

#[derive(Debug)]
pub struct Store {
    inner: RwLock<HashMap<String, Value>>
}

impl Store {
    pub fn new() -> Self {
        Store {
            inner: RwLock::new(HashMap::new())
        }
    }

    // only put if the incoming value is newer (based on Lamport timestamp and node_id)
    pub async fn put(&self, key: String, value: String, ts: u64, node_id: u64) {
        let incoming = Value { data: Some(value), ts, node_id };
        let mut map = self.inner.write().await;

        let current = map.get(&key);
        if incoming.is_newer_than(current) {
            map.insert(key, incoming);
        }
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let map = self.inner.read().await;
        map.get(key).cloned()
    }

    // delete writes None into the map to represent a tombstone
    pub async fn delete(&self, key: &str, ts: u64, node_id: u64) -> Option<Value> {
        let incoming = Value { data: None, ts, node_id };
        let mut map = self.inner.write().await;

        match map.get(key) {
            None => {
                map.insert(key.to_string(), incoming);
                None
            },
            Some(existing) => {
                if incoming.is_newer_than(Some(existing)) {
                    let prev = map.insert(key.to_string(), incoming);
                    prev
                } else {
                    None
                }
            }
        }
    }
}