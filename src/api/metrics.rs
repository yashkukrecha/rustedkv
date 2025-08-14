use prometheus::{Encoder, TextEncoder, Registry, IntCounterVec};

#[derive(Debug, Clone)]
pub struct Metrics {
    pub registry: Registry,
    pub kv_ops: IntCounterVec,
    pub requests: IntCounterVec,
    pub errors: IntCounterVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let kv_ops = IntCounterVec::new(
            prometheus::Opts::new("kv_ops", "Key-Value Operations"),
            &["op"],
        ).unwrap();
        let requests = IntCounterVec::new(
            prometheus::Opts::new("requests", "Total API Requests"),
            &["method", "path", "status"],
        ).unwrap();
        let errors = IntCounterVec::new(
            prometheus::Opts::new("errors", "Total API Errors"),
            &["kind"],
        ).unwrap();

        registry.register(Box::new(kv_ops.clone())).unwrap();
        registry.register(Box::new(requests.clone())).unwrap();
        registry.register(Box::new(errors.clone())).unwrap();

        Self { registry, kv_ops, requests, errors }
    }
}