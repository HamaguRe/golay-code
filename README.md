# Golay Code implementation

ゴレイ符号の実装．3bitまでの誤り訂正と4bitまでの誤り検出が可能．

## Example of use

Cargo.toml

```toml
[dependencies]
goray_code = {git = "https://github.com/HamaguRe/quaternion.git"}
```

src/main.rs

```rust
use goray_code;

main() {
    let data = 0b001001110011;
    let tx = golay_code::encode(data);

    let e  = 0b0000_0001_0000_0000_1000_0100;  // 3bit error
    let rx = tx ^ e;

    let (flag, rec) = golay_code::ecc(rx);
    assert_eq!(flag, true);
    assert_eq!(data, golay_code::decode(rec));
}
```
