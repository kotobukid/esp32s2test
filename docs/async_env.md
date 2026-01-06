# ESP32-S2 Rust 非同期開発環境 (v1.0.0世代) の構築メモ

ESP32-S2 (シングルコア) において、`esp-hal` v1.0.0 をベースに非同期処理 (async/await) や NeoPixel 制御を組み合わせる際の重要なポイントをまとめます。

## 1. 推奨される依存関係の組み合わせ (Cargo.toml)

`esp-hal` v1.0.0 リリース直後の環境では、ライブラリ間の不整合が原因で `portable-atomic` クレートがコンパイルエラー（`critical-section` との衝突）を起こすことがあります。

現在、`esp-hal-embassy` クレートを独立して追加すると、内部で古いバージョンの `esp-hal` が引き込まれ、アトミック操作のフラグ衝突エラーが発生するケースがあります。

### 安定した組み合わせの例

```toml
[dependencies]
# unstableは非同期メソッド (wait_for_any_edge等) の利用に必要
esp-hal = { version = "1.0.0", features = ["esp32s2", "unstable"] }
esp-println = { version = "0.13.0", features = ["esp32s2"] }
esp-backtrace = { version = "0.15.0", features = ["esp32s2", "panic-handler", "exception-handler", "println"] }

# 非同期エグゼキュータとタイマー（将来の完全移行用）
embassy-executor = { version = "0.7.0", features = ["task-arena-size-12288"] }
embassy-time = { version = "0.4.0", features = ["generic-queue-8"] }
static_cell = "2.1.0"

# SmartLEDs (NeoPixel)
smart-leds = "0.4"
esp-hal-smartled = { version = "0.17.0", features = ["esp32s2"] }
```

## 2. 非同期処理 (embassy) への移行について

### 現状の制約
`esp-hal-embassy` v0.7.0 以上が `esp-hal` v1.0.0 に対応していますが、Cargoの依存関係解決において内部フィーチャー（`__esp_hal_embassy`）の不整合が発生することがあります。

### 移行の道筋
1. **Blocking モード**: 現在の実装。依存関係の衝突を避けつつ、`unstable` フィーチャーにより将来の非同期化への互換性を確保。
2. **非同期化（移行後）**: `#[esp_hal::main]` を `async fn` にし、`pin17.wait_for_any_edge().await` を利用する。これにより、ボタン押下を待機している間、CPUを他のタスクに明け渡すことが可能になる。

## 3. 実装上の注意点

### SmartLedsAdapter の初期化 (v0.17.0以降)
RMTバッファをユーザー側で明示的に管理する形式に変更されています。`smart_led_buffer!` マクロを使用してバッファを確保し、その参照をアダプタに渡します。

```rust
const LEDS: usize = 1;
let mut rmt_buffer = smart_led_buffer!(LEDS);
let mut neopixel = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO18, &mut rmt_buffer);
```

### GPIO 17 (タクトスイッチ) の検知
非同期モードに移行した際は、以下のコードで効率的に待機できます。

```rust
loop {
    // ピンの状態が変化するまで非同期に待機（CPU効率が良い）
    pin17.wait_for_any_edge().await;
    
    if pin17.is_high() {
        // 点灯処理...
    } else {
        // 消灯処理...
    }
}
```

## 4. トラブルシューティング

- **portable-atomic エラー**: `critical-section` フィーチャーを明示的に有効化している箇所がないか確認してください。シングルコアの ESP32-S2 では `esp-hal` が提供する同期機構に任せるのが正解です。
- **feature 不整合**: `esp-hal` v1.0.0 以降、一部の内部フィーチャー名が変更されています。ビルドエラーが出る場合は、周辺クレート（`esp-println` など）が v1.0 対応版になっているか確認してください。
