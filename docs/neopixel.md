# ESP32-S2（Rust/esp-hal）で NeoPixel（RGB@IO18）を点滅させるメモ

## TL;DR（結論）

- ボードのシルクが `RGB@IO18` の場合、これは単純なGPIO LEDではなく **NeoPixel (WS2812/SK6812系)** の可能性が高い。
- NeoPixelは **High/Lowの点滅では制御できず**、専用のタイミング波形（RMT等）が必要。
- `esp-hal-smartled2` を使うと比較的ラクに点灯できるが、以下が重要:
    - `esp-hal` の `unstable` feature が必要
    - `Rmt::new()` は周波数 `Rate` が必須で `Result` を返す
    - `SmartLedsAdapter` の **バッファサイズ（型引数）** が小さいと `write()` が失敗し、`unwrap()` で止まる

---

## 背景：なぜGPIOの点滅で光らなかったのか

NeoPixelは「最後に受け取った色データ」を保持するタイプのLEDで、データ線には 800kHz相当の厳密なパルス幅でビット列を送る必要がある。

そのため、

- `Output::new(...).set_high()/set_low()` のようなGPIO操作では
- LEDへ「色データ」が届かず
- 以前（MicroPython等）に送った色が残る、または無反応に見える

---

## 使用クレート構成（Cargo.toml）

今回の最小構成（プロジェクト側）:

```
toml
[dependencies]
esp-hal = { version = "~1.0", features = ["esp32s2", "unstable"] }
esp-bootloader-esp-idf = { version = "0.4.0", features = ["esp32s2"] }
critical-section = "1.2.0"

# NeoPixel(RMT)制御
smart-leds = "0.4"
esp-hal-smartled2 = "0.27.0"

# 実行ログ出力（デバッグ用）
esp-println = { version = "0.16.1", features = ["esp32s2"] }
```

### ハマり1：`unstable` が必要

`esp-hal-smartled2` は `esp-hal` の `unstable` API（主にRMT周辺）を使うため、`esp-hal` に `unstable` feature を付けないと
build.rs が止める:

> The `unstable` feature is required by a dependent crate but is not enabled.

対処：`esp-hal` に `unstable` を追加する。

---

## 動作するサンプル（GPIO18のNeoPixelを点滅）

このサンプルは GPIO18 の NeoPixel（LED 1個）を「白(弱め) ↔ 消灯」で点滅させる。  
ログも出すので「動いているのにLEDが無反応」の切り分けに使える。

```
rust
#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::time::{Duration, Instant, Rate};

use esp_hal::Blocking;
use esp_hal::rmt::Rmt;

use esp_hal_smartled::color_order::Grb;
use esp_hal_smartled::{SmartLedsAdapter, Ws2812bTiming};
use smart_leds::{SmartLedsWrite, RGB8};

use esp_println::println;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
// LED個数（今回1個）
const LEDS: usize = 1;

    // 超重要：SmartLedsAdapter の内部バッファサイズ
    // 小さすぎると write() が失敗する可能性があるため、まずは 64 程度にする
    const RMT_BUFFER_SIZE: usize = 64;

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    println!("boot: start");

    // ハマり2：Rmt::new は周波数が必須 & Result を返す
    // ESP32-S2は 80MHz が基準として扱われることが多い
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    // ハマり3：SmartLedsAdapter::new は「channel creator + pin」で作る
    let mut neopixel: SmartLedsAdapter<'_, RMT_BUFFER_SIZE, Blocking, RGB8, Grb, Ws2812bTiming> =
        SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO18).unwrap();

    println!("boot: neopixel adapter ready");

    // 明るさ控えめの白（眩しいので 12/255 程度）
    let white: [RGB8; LEDS] = [RGB8 { r: 12, g: 12, b: 12 }];
    let off: [RGB8; LEDS] = [RGB8 { r: 0, g: 0, b: 0 }];

    loop {
        println!("set: white");
        if let Err(e) = neopixel.write(white.iter().copied()) {
            println!("write error (white): {:?}", e);
        }

        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(500) {}

        println!("set: off");
        if let Err(e) = neopixel.write(off.iter().copied()) {
            println!("write error (off): {:?}", e);
        }

        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_millis(500) {}
    }
}
```

---

## 今回の主なエラーと原因まとめ

### エラーA：`Io` が private

- `esp-hal` のAPIは版で変化する。GPIOまわりの初期化で古い例を引くと `Io` がprivate等の差分が出る。
- NeoPixelの場合そもそもGPIOの `Output` 点滅では目的に合わない（RMT等へ切り替えるべき）。

### エラーB：`Output::new` の引数が足りない

- `esp-hal` では `Output::new(pin, level, config)` の形（引数が3つ）になっている版がある。

### エラーC：`unstable` が必要

- `esp-hal-smartled2` が `esp-hal` の `unstable` APIを使うため。
- 対処：`esp-hal` の features に `"unstable"` を追加。

### エラーD：`Rmt::new(peripherals.RMT)` が引数不足 & `Result` で `.channel0` が生えない

- `Rmt::new(peripheral, frequency: Rate) -> Result<...>` が正しい。
- 対処：`Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap()` のようにする。

### エラーE（今回の核心）：ログが `set: white` で止まる

症状：

- `boot: start`
- `boot: neopixel adapter ready`
- `set: white`
- 以降ログが増えない（ただしリセットログは出る）

原因（断定）：

- `neopixel.write(...).unwrap()` が `Err` を返し panic → `panic_handler` が無限ループ。
- その `Err` の主因は **SmartLedsAdapter のバッファサイズが小さすぎた**こと。
    - `SmartLedsAdapter<'_, 1, ...>` の `1` は「LED個数」ではなく「内部バッファサイズ」。
    - `RMT_BUFFER_SIZE` を十分に増やす（例：64）ことで `write()` が成功するようになり、実機で点滅を確認できた。

対処：

- `RMT_BUFFER_SIZE` を増やす
- `unwrap()` をやめて `if let Err(e)` でログに出す（デバッグ時）

---

## よくある追加調整

### 明るさ調整

- 値を小さくする（例：`RGB8 { r: 12, g: 12, b: 12 }`）
- `smart-leds` の brightness ヘルパを使う手もあるが、まずは直値で十分。

### 色順（GRB/RGB）

- 多くのNeoPixelは GRB。違う場合は `color_order` を変更する必要がある。
- ただし「白が点く」なら色順問題ではなく、基本の送信自体は成功している。

### LED種別（WS2812B vs SK6812）

- 無反応ならタイミング型を変える（`Ws2812bTiming` → `Ws2812Timing` / `Sk68xxTiming` など）を試す。

---

## 参考：MicroPythonで動いていた例

```
python
from machine import Pin
from neopixel import NeoPixel

np = NeoPixel(Pin(18), 1)
np[0] = (255, 0, 0)
np.write()
```

これが動いていた＝少なくとも「データ線はGPIO18」かつ「NeoPixel系」である可能性が高い。
