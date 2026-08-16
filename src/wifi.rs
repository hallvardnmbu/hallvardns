use alloc::string::ToString;
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Timer};
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{Config, Interface, WifiController};

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub fn station_config() -> Config {
    Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.to_string()),
    )
}

pub async fn wait_for_connection(stack: Stack<'_>) {
    while !stack.is_link_up() {
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        if let Some(_) = stack.config_v4() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    loop {
        if controller.is_connected() {
            let _ = controller.wait_for_disconnect_async().await.ok();
            Timer::after(Duration::from_millis(5000)).await;
        }

        if let Err(_) = controller.set_config(&station_config()) {
            Timer::after(Duration::from_millis(5000)).await;
            continue;
        }

        match controller.connect_async().await {
            Ok(_) => (),
            Err(_) => {
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}
