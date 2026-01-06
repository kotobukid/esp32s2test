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
use esp_hal::rng::Rng;
use esp_hal::time::Rate;

use esp_hal::rmt::Rmt;

use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{RGB8, SmartLedsWrite};

use esp_println::println;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("panic: {:?}", info);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

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

#[esp_hal::main]
fn main() -> ! {
    const LEDS: usize = 1;

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("boot: start (async wait mode)");

    let rng = Rng::new();
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let mut rmt_buffer = smart_led_buffer!(LEDS);
    let mut neopixel = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO18, &mut rmt_buffer);

    let pin17 = Input::new(
        peripherals.GPIO17,
        InputConfig::default().with_pull(Pull::Up),
    );

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

    let _ = neopixel.write(off.iter().copied());

    println!("boot: neopixel off");

    let mut last_pin17_state = pin17.is_high();

    loop {
        // unstable フィーチャーにより wait_for_any_edge が利用可能だが、
        // 呼び出すには async コンテキストが必要。
        // ここでは Blocking なループを維持しつつ、将来の拡張性を docs に残す。
        let is_high = pin17.is_high();

        if is_high != last_pin17_state {
            last_pin17_state = is_high;
            if !is_high {
                let color_index = (rng.random() as usize) % colors.len();
                let mut color = colors[color_index];

                let is_half = rng.random().is_multiple_of(2);
                if is_half {
                    color = color.half_brightness();
                }

                let current_color = [color.to_rgb8()];

                println!(
                    "set: color index {}, brightness: {}",
                    color_index,
                    if is_half { "half" } else { "full" }
                );
                let _ = neopixel.write(current_color.iter().copied());
            } else {
                println!("set: off");
                let _ = neopixel.write(off.iter().copied());
            }
        }
    }
}
