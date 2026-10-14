//! Extended binary Golay code [24, 12, 8].
//!
//! Each 12-bit data word is encoded into a 24-bit codeword.
//! Since the minimum distance is 8, any error of up to 3 bits can be corrected
//! and any 4-bit error can be detected.
//!
//! The code is systematic: the upper 12 bits of a codeword are the data itself
//! and the lower 12 bits are parity bits.
//!
//! This crate is `no_std` and does not allocate.
//!
//! # Example
//!
//! ```
//! let data: u16 = 0b0010_0111_0011;  // original data (12 bits)
//! let tx: u32 = golay_code::encode(data);  // encode into a 24-bit codeword
//!
//! let e  = 0b0000_0001_0000_0000_1000_0100;  // 3-bit error
//! let rx = tx ^ e;  // received word with some bits flipped
//!
//! // error detection & correction (when you need the corrected codeword)
//! let corrected: Option<u32> = golay_code::ecc(rx);
//! assert_eq!(Some(tx), corrected);
//!
//! // correct errors and extract the 12-bit data
//! assert_eq!(Some(data), golay_code::decode(rx));
//! ```


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

/// The 12 × 12 matrix `B`, where `G = [I | B]` and `transpose(H) = [B; I]`.
///
/// Each element is one row, with column 0 at the most significant of the 12 bits.
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

/// Encodes 12-bit data into a 24-bit codeword.
///
/// Only the lower 12 bits of `a` are used; the upper 4 bits are ignored.
///
/// The codeword occupies the lower 24 bits of the return value
/// (data in bits 23..12, parity in bits 11..0), and the upper 8 bits are 0.
#[inline]
pub fn encode(a: u16) -> u32 {
    // 符号語の上位12bitはデータそのもので，下位12bitがパリティ（データ × B）．
    let a = (a & 0xFFF) as u32;
    (a << 12) | mul_vec_mat(a, &B)
}

/// Detects and corrects errors in a received word, returning the corrected codeword.
///
/// Only the lower 24 bits of `r` are used; the upper 8 bits are ignored.
///
/// * Up to 3 bit errors: returns `Some` with the corrected codeword (upper 8 bits are 0).
/// * Exactly 4 bit errors: returns `None`.
/// * 5 or more bit errors: the result is unreliable. It may be `None`,
///   or `Some` with a wrong codeword (miscorrection).
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

    None  // 4bit error
}

/// Corrects errors in a received word with [`ecc`] and returns the 12-bit data.
///
/// The data is returned in the lower 12 bits, and the upper 4 bits are 0.
/// Returns `None` whenever [`ecc`] does, so the same caveats apply to 5 or more bit errors.
#[inline]
pub fn decode(code: u32) -> Option<u16> {
    // 生成行列からわかるように，元データは上位12bitに入っている．
    ecc(code).map(|t| ((t >> 12) & 0xFFF) as u16)
}

/// Returns the Hamming weight (the number of 1 bits) of a syndrome.
#[inline]
fn weight(s: u32) -> u32 {
    s.count_ones()
}

/// Multiplies the row vector `v` by the matrix `m` over GF(2).
///
/// `v` has `m.len()` bits, with its most significant bit corresponding to row 0 of `m`.
/// The result is the XOR of the rows of `m` whose corresponding bit in `v` is 1.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
