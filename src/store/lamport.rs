use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct LamportClock {
    counter: AtomicU64,
}

impl LamportClock {
    pub fn new() -> Self {
        LamportClock {
            counter: AtomicU64::new(0),
        }
    }

    pub fn tick_send(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn tick_recv(&self, ts: u64) -> u64 {
        loop {
            let current = self.counter.load(Ordering::SeqCst);
            let next = current.max(ts) + 1;
            if self.counter.compare_exchange_weak(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                return next;
            }
        }
    }

    pub fn tick_observe(&self, ts: u64) {
        loop {
            let current = self.counter.load(Ordering::SeqCst);
            if ts <= current {
                return;
            }

            if self.counter.compare_exchange_weak(current, ts, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                return;
            }
        }
    }

    pub fn tick_now(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}