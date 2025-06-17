use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Value {
    pub data: String,
    pub clock: u64
}

pub struct Store {
    inner: RwLock<HashMap<String, Value>>
}

impl Store {
    pub fn new() -> Self {
        Store {
            inner: RwLock::new(HashMap::new())
        }
    }

    pub fn put(&self, key: String, value: String, clock: u64) {
        let mut map = self.inner.write().unwrap();
        map.insert(key, Value { data: value, clock });
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        let map = self.inner.read().unwrap();
        map.get(key).cloned()
    }
}