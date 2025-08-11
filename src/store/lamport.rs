use tokio::sync::Mutex;

pub type LamportClock = Mutex<u64>;

pub async fn increment_clock(clock: &LamportClock) -> u64 {
    let mut clk = clock.lock().await;
    *clk += 1;
    *clk
}

pub async fn read_clock(clock: &LamportClock) -> u64 {
    *clock.lock().await
}