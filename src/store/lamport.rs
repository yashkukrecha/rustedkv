use std::sync::Mutex;

pub type LamportClock = Mutex<u64>;

pub fn increment_clock(clock: &LamportClock) -> u64 {
    let mut clk = clock.lock().unwrap();
    *clk += 1;
    *clk
}

pub fn read_clock(clock: &LamportClock) -> u64 {
    *clock.lock().unwrap();
}