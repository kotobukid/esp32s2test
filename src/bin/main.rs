#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::time::{Duration, Instant, Rate};

use esp_hal::Blocking;
use esp_hal::rmt::Rmt;

use esp_hal_smartled::color_order::Grb;
use esp_hal_smartled::{SmartLedsAdapter, Ws2812Timing};

use esp_println::println;
use smart_leds::{RGB8, SmartLedsWrite};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("boot: start");

    // RMT初期化：周波数が必須 & Resultが返る
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    // NeoPixel(WS2812) + 色順GRB を想定（違ったら後述）
    let mut neopixel: SmartLedsAdapter<'_, 1, Blocking, RGB8, Grb, Ws2812Timing> =
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO18).unwrap();

    println!("boot: neopixel adapter ready");

    let on = [RGB8 { r: 0, g: 255, b: 0 }];
    let off = [RGB8 { r: 0, g: 0, b: 0 }];

    loop {
        println!("set: green");
        neopixel.write(on.iter().copied()).ok();
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(500) {}

        println!("set: off");
        neopixel.write(off.iter().copied()).ok();
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(500) {}
    }
}
