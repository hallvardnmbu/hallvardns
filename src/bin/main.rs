#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_net::StackResources;
use esp_hal::{
    clock::CpuClock,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_radio::wifi::ControllerConfig;
use static_cell::StaticCell;

use hallvardns::{dns, wifi};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    esp_println::println!("PANIC: {info}");
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals =
        esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    // Reserve 96 KiB RAM as heap for the global allocator.
    esp_alloc::heap_allocator!(
        #[esp_hal::ram(reclaimed)]
        size: 96 * 1024
    );

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let interrupts =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(
            peripherals.SW_INTERRUPT,
        );

    esp_rtos::start(timg0.timer0, interrupts.software_interrupt0);

    let (controller, interfaces) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(wifi::config()),
    )
    .unwrap();

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let rng = Rng::new();
    let seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let (stack, runner) = embassy_net::new(
        interfaces.station,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(wifi::connection(controller).unwrap());
    spawner.spawn(wifi::net_task(runner).unwrap());

    while stack.config_v4().is_none() {
        embassy_time::Timer::after_secs(1).await;
    }

    dns::run(stack).await;
}
