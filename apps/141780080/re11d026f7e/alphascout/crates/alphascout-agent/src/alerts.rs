use tokio::time::{Duration, sleep};

pub async fn start_alert_loop() {
    tokio::spawn(async move {
        loop {
            println!("🔔 AlphaScout scanning markets...");

            sleep(Duration::from_secs(60)).await;
        }
    });
}
