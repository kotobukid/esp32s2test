#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::main;
use esp_hal::rng::Rng;
use esp_hal::time::{Duration, Instant, Rate};

use esp_hal::Blocking;
use esp_hal::rmt::Rmt;

use esp_hal_smartled::color_order::Grb;
use esp_hal_smartled::{SmartLedsAdapter, Ws2812bTiming};
use smart_leds::{RGB8, SmartLedsWrite};

#[derive(Clone, Copy)]
struct MyColor(u8, u8, u8);

impl MyColor {
    fn half_brightness(self) -> Self {
        MyColor(self.0 / 2, self.1 / 2, self.2 / 2)
    }

    fn to_rgb8(self) -> RGB8 {
        RGB8 {
            r: self.0,
            g: self.1,
            b: self.2,
        }
    }
}

use esp_println::println;

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
    const LEDS: usize = 1;
    const RMT_BUFFER_SIZE: usize = 64;

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("boot: start");

    let rng = Rng::new();
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let mut neopixel: SmartLedsAdapter<'_, RMT_BUFFER_SIZE, Blocking, RGB8, Grb, Ws2812bTiming> =
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO18).unwrap();

    let pin17 = Input::new(
        peripherals.GPIO17,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut last_pin17_state = pin17.is_high();

    println!("boot: neopixel adapter ready");

    let colors = [
        MyColor(12, 24, 12), // 白
        MyColor(48, 0, 0),   // 赤
        MyColor(0, 48, 0),   // 緑
        MyColor(0, 0, 48),   // 青
        MyColor(24, 24, 0),  // 黄
        MyColor(24, 0, 24),  // 紫
        MyColor(0, 24, 24),  // 水色
    ];
    let off = [RGB8 { r: 0, g: 0, b: 0 }];

    if let Err(e) = neopixel.write(off.iter().copied()) {
        println!("write error (off): {:?}", e);
    }

    println!("boot: neopixel off");

    loop {
        let current_pin17_state = pin17.is_high();

        if current_pin17_state != last_pin17_state {
            println!("pin17 state changed: {}", current_pin17_state);
            last_pin17_state = current_pin17_state;

            if !last_pin17_state {
                let color_index = (rng.random() as usize) % colors.len();
                let mut color = colors[color_index];

                // 50%の確率で明るさを半分にする
                let is_half = (rng.random() % 2) == 0;
                if is_half {
                    color = color.half_brightness();
                }

                let current_color = [color.to_rgb8()];

                println!(
                    "set: color index {}, brightness: {}",
                    color_index,
                    if is_half { "half" } else { "full" }
                );
                if let Err(e) = neopixel.write(current_color.iter().copied()) {
                    println!("write error (color): {:?}", e);
                }

                let t0 = Instant::now();
                while t0.elapsed() < Duration::from_millis(500) {}
            } else {
                println!("set: off");
                if let Err(e) = neopixel.write(off.iter().copied()) {
                    println!("write error (off): {:?}", e);
                }

                let t0 = Instant::now();
                while t0.elapsed() < Duration::from_millis(500) {}
            }
        }
    }
}
