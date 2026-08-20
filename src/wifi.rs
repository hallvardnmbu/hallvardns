extern crate alloc;

use alloc::string::ToString;
use embassy_net::Runner;
use embassy_time::Timer;
use esp_radio::wifi::{
    sta::StationConfig,
    Config,
    Interface,
    WifiController,
};

pub fn config() -> Config {
    let ssid: &str = env!("SSID");
    let password: &str = env!("PASSWORD");

    Config::Station(
        StationConfig::default()
            .with_ssid(ssid)
            .with_password(password.to_string()),
    )
}

/// Connect to the WiFi, and reconnect on disconnects.
#[embassy_executor::task]
pub async fn connect(mut controller: WifiController<'static>) {
    loop {
        if !controller.is_connected() {
            let _ = controller.connect_async().await;
        }

        // Re-enter loop (and try to reconnect) on disconnect.
        if controller.is_connected() {
            let _ = controller.wait_for_disconnect_async().await;
        }

        Timer::after_secs(2).await;
    }
}

/// Set up the networking stack.
#[embassy_executor::task]
pub async fn network(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}
