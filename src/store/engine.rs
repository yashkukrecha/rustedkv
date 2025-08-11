use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Value {
    pub data: String,
    pub clock: u64
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

    pub async fn put(&self, key: String, value: String, clock: u64) {
        let mut map = self.inner.write().await;
        map.insert(key, Value { data: value, clock });
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let map = self.inner.read().await;
        map.get(key).cloned()
    }
}