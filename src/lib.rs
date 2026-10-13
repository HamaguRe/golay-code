//! 拡張２元ゴレイ符号を実装
//!
//! 3bitまでのエラー訂正と4bitまでの誤り検出が可能．


// 生成行列`G` (12bit × 24bit) と検査行列の転置`H_T` (24bit × 12bit) は，
// 単位行列`I`と12bit × 12bitの行列`B`を使って次のように表せる．
//
// ```text
// G   = [ I | B ]
//
// H_T = [ B ]
//       [ I ]
// ```
//
// 単位行列の部分は掛けても値をそのまま写すだけなので，実装では`B`だけを持って計算している．

#![no_std]

/// 生成行列の右半分（検査行列の転置の上半分）(12bit × 12bit)
const B: [u32; 12] = [
    0b100111110001,
    0b010011111010,
    0b001001111101,
    0b100100111110,
    0b110010011101,
    0b111001001110,
    0b111100100101,
    0b111110010010,
    0b011111001001,
    0b001111100110,
    0b010101010111,
    0b101010101011,
];

/// 12bitのデータを24bitの符号語に変換する．
/// 
/// データは下位12bitに入れておく．
/// 上位4bitは見ないので何でも良い．
/// 
/// 変換後の符号語は下位24bitに入っている．
#[inline]
pub fn encode(a: u16) -> u32 {
    // 符号語の上位12bitはデータそのもので，下位12bitがパリティ（データ × B）．
    let a = (a & 0xFFF) as u32;
    (a << 12) | mul_vec_mat(a, &B)
}

/// 受信語のエラー検出と訂正を行う．
/// 
/// 受信語`r`は下位24bitに入れておく．上位8bitは無視する．
///
/// 3bitまでの誤りは訂正して，訂正後の符号語を`Some`で返す（上位8bitは0）．
/// 4bitの誤りを検出した場合は`None`を返す．
///
/// 5bit以上の誤りでは`Some`を返す場合もあるが，正しく訂正できているわけではない．
#[inline]
pub fn ecc(r: u32) -> Option<u32> {
    // ゴレイ符号に関係無い部分は消す．
    let r = r & 0xFFFFFF;

    // 1つめのシンドローム
    // シンドロームが0の場合は，誤りなしもしくは検出できない状態．
    // H_Tの下半分は単位行列なので，rの下位12bitはそのままXORすれば良い．
    let s = mul_vec_mat(r >> 12, &B) ^ (r & 0xFFF);
    if weight(s) <= 3 {
        return Some(r ^ s);
    }
    for (i, b_line) in B.iter().enumerate() {
        let tmp = s ^ *b_line;
        if weight(tmp) <= 2 {
            let e = (0x800000 >> i) | tmp;
            return Some(r ^ e);
        }
    }

    // 2つめのシンドローム
    let sh = mul_vec_mat(s, &B);
    if weight(sh) <= 3 {
        return Some(r ^ (sh << 12));
    }
    for (i, b_line) in B.iter().enumerate() {
        let tmp = sh ^ *b_line;
        if weight(tmp) <= 2 {
            let e = (tmp << 12) | (0x800 >> i);
            return Some(r ^ e);
        }
    }

    None  // 4bitエラー
}

/// 受信データからデータを取り出す．
/// 
/// `ecc`で誤り訂正してから，12bitのデータを取り出す．
/// 4bitの誤りを検出した場合は`None`を返す．
///
/// 返り値のデータは下位12bitに入っている．
/// 上位4bitは必ず0．
#[inline]
pub fn decode(code: u32) -> Option<u16> {
    // 生成行列からわかるように，元データは上位12bitに入っている．
    ecc(code).map(|t| ((t >> 12) & 0xFFF) as u16)
}

/// シンドロームの重みを計算する（1になっているビットを数える）．
#[inline]
fn weight(s: u32) -> u32 {
    s.count_ones()
}

/// 行ベクトル`v`と行列`m`の積を計算する（加算はXOR）．
///
/// `v`は下位`m.len()`bitを使い，最上位のビットが`m`の0行目に対応する．
/// ビット演算で処理するために，`v`のビットが1である行をXORで足し合わせている．
#[inline]
fn mul_vec_mat(v: u32, m: &[u32]) -> u32 {
    let n = m.len();
    let mut acc = 0;
    for (i, line) in m.iter().enumerate() {
        // 左のビットから順に見ていって，そのビットが1なら全ビット1にしたbit_maskを作る．
        // 0u32からwrapping_subで-1すると0xFFFF_FFFFになることを利用している．
        let bit_mask = 0u32.wrapping_sub((v >> (n - 1 - i)) & 1);
        acc ^= bit_mask & *line;
    }
    acc
}

#[test]
fn test() {
    let tx = 0b100110001101;  // 任意のデータ（12bit）
    let encoded = encode(tx);

    // 全パターンチェック
    for i in 0..24 {
        for j in 0..24 {
            for k in 0..24 {
                for l in 0..24 {                
                    // エラービット
                    let error = (1 << i) | (1 << j) | (1 << k) | (1 << l);

                    // errorでビット反転させる．
                    let rx = encoded ^ error;

                    // エラー検出&訂正してデータを取り出す
                    let decoded = decode(rx);
                    
                    // エラービットが4bit未満なら全て訂正可能
                    let error_bits = error.count_ones();
                    if error_bits < 4 {
                        assert_eq!(Some(tx), decoded);
                    } else if error_bits == 4 {
                        assert_eq!(None, decoded);
                    }
                }
            }
        }
    }
}