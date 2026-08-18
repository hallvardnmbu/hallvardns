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

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    loop {
        if !controller.is_connected() {
            let _ = controller.connect_async().await;
        }

        if controller.is_connected() {
            let _ = controller.wait_for_disconnect_async().await;
        }

        Timer::after_secs(2).await;
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}
