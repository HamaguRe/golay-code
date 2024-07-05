# Golay Code implementation

拡張2元ゴレイ符号の実装．
12bitの元データに対して，3bitまでの誤り訂正と4bitまでの誤り検出が可能．

## Example of use

Cargo.toml

```toml
[dependencies]
goray-code = {git = "https://github.com/HamaguRe/golay-code"}
```

src/main.rs

```rust
use goray_code;

main() {
    let data: u16 = 0b0010_0111_0011;  // 元データ
    let tx: u32 = golay_code::encode(data);  // 24bitの符合語に変換

    let e  = 0b0000_0001_0000_0000_1000_0100;  // 3bitエラー
    let rx = tx ^ e;  // 一部ビット反転させた符号語を受信語とする

    let corrected = golay_code::ecc(rx);  // 誤り検出 & 訂正
    assert_eq!( data, golay_code::decode(corrected.unwrap()) );
}
```
