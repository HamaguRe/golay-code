//! 拡張２元ゴレイ符号を実装
//!
//! 3bitまでのエラー訂正と4bitまでの誤り検出が可能．

#![no_std]

/// 検査行列の転置 (24bit × 12bit)
const H_T: [u32; 24] = [
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
    0b100000000000,
    0b010000000000,
    0b001000000000,
    0b000100000000,
    0b000010000000,
    0b000001000000,
    0b000000100000,
    0b000000010000,
    0b000000001000,
    0b000000000100,
    0b000000000010,
    0b000000000001,
];

/// 生成行列 (12bit × 24bit)
const G: [u32; 12] = [
    0b100000000000_100111110001,
    0b010000000000_010011111010,
    0b001000000000_001001111101,
    0b000100000000_100100111110,
    0b000010000000_110010011101,
    0b000001000000_111001001110,
    0b000000100000_111100100101,
    0b000000010000_111110010010,
    0b000000001000_011111001001,
    0b000000000100_001111100110,
    0b000000000010_010101010111,
    0b000000000001_101010101011,
];

/// 12bitのデータを24bitの符合語に変換する．
/// 
/// データは下位12bitに入れておく．
/// 上位4bitは見ないので何でも良い．
/// 
/// 変換後の符号語は下位24bitに入っている．
#[inline]
pub fn encode(a: u16) -> u32 {
    let a = a as u32;
    let mut code = 0;  // 符号語
    // aベクトルとG行列の積（加算はXOR）
    // ビット演算で処理するために通常の行列積を転置したような状態で計算している
    for (i, g_line) in G.iter().enumerate() {
        // // 左のビットから順に見ていって，そのビットが1なら24bitすべて1にする
        let a_bit = ((a >> (11 - i)) & 1) * 0xFFFFFF;
        code ^= a_bit & *g_line;
    }
    code
}

/// 受信語のエラー検出と訂正を行う．
/// 
/// * `r`: 受信した符号語（下位24bit）
/// * return: `Option<u32>`
///     * `code`: 誤り訂正した受信語．
///     * 誤りを訂正できたらSome(code)，4bit誤りの場合はNoneを返す．
///     * 5bit以上のエラーではSome(code)を返す場合もあるが，正しく訂正できているわけではない．
///     * 4bit以上反転していてもエラービットが全て下位12bitにあれば元データは問題なく復号できる．
#[inline]
pub fn ecc(r: u32) -> Option<u32> {
    // 1つめのシンドローム
    let mut s: u32 = 0;
    // rベクトルとH_T行列の積（加算はXOR）
    for (i, h_t_line) in H_T.iter().enumerate() {
        // 左のビットから順に見ていって，そのビットが1なら12bitすべて1にする
        let r_bit = ((r >> (23 - i)) & 1) * 0xFFF;
        s ^= r_bit & *h_t_line;
    }

    // シンドロームが0なら誤りなし（もしくは検出できない）．
    // weightの計算が少し重いのでここで返してしまう．
    if s == 0 {
        return Some(r);
    }

    if weight(s) <= 3 {
        return Some(r ^ s);
    } else {
        for (i, h_t_line) in H_T.iter().take(12).enumerate() {
            let tmp = s ^ *h_t_line;
            if weight(tmp) <= 2 {
                let e = (0x800000 >> i) | tmp;
                //let e = G[i] ^ s;  // こう書いても同じ
                return Some(r ^ e);
            }
        }
    }

    // 2つめのシンドローム
    let mut sh = 0;
    for (i, h_t_line) in H_T.iter().take(12).enumerate() {
        let s_bit = ((s >> (11 - i)) & 1) * 0xFFF;
        sh ^= s_bit & *h_t_line;
    }
    if weight(sh) <= 3 {
        return Some(r ^ (sh << 12));
    } else {
        for (i, h_t_line) in H_T.iter().take(12).enumerate() {
            let tmp = sh ^ *h_t_line;
            if weight(tmp) <= 2 {
                let e = (tmp << 12) | (0x800 >> i);
                return Some(r ^ e);
            }
        }
    }

    None  // 4bitエラー
}

/// 符合語からデータを取り出す．
/// 
/// 返り値のデータは下位12bitに入っている．
/// 上位4bitは必ず0．
#[inline]
pub fn decode(code: u32) -> u16 {
    // 生成行列からわかるように，元データは上位12bitに入っている．
    ((code >> 12) & 0xFFF) as u16
}

/// シンドロームの重みを計算する（1になっているビットを数える）．
#[inline]
fn weight(s: u32) -> u32 {
    s.count_ones()
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

                    // エラー検出&訂正
                    let corrected = ecc(rx);
                    
                    // エラービットが4bit未満なら全て訂正可能
                    let error_bits = error.count_ones();
                    if error_bits < 4 {
                        assert_eq!(tx, decode(corrected.unwrap()));
                    } else if error_bits == 4 {
                        assert_eq!(None, corrected);
                    }
                }
            }
        }
    }
}