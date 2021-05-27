// 拡張２元ゴレイ符号(Golay Code)を実装
//
// 3bitまでのエラー訂正と4bitまでの誤り検出が可能．

/// 検査行列の転置
const H_T: [u16; 24] = [
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

/// 生成行列
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

fn main() {
    let tx = 0b000110001101;
    println!("tx: {:<012b}", tx);
    let encoded = encode(tx);
    println!("encoded: {:<024b}", encoded);

    // 意図的にビット反転させたデータ
    let error: u32 = 0b0100_0100_0001_0000_0000_0000;  // エラービット
    println!("error: {:<024b}, bits: {}", error, error.count_ones());
    let rx = encoded ^ error;  // errorでビット反転させる．
    println!("rx: {:<024b}", rx);
    let (flag, corrected) = ecc(rx);
    println!("corrected: {:<024b}, flag: {:?}", corrected, flag);

    // 符号語から元のデータを取り出す
    println!("tx (recover): {:<012b}", decode(corrected));
    println!("{:?}", tx == decode(corrected));

    // 全パターンチェック
    /*
    for i in 0..24 {
        for j in 0..24 {
            for k in 0..24 {
                for l in 0..24 {                
                    // 意図的にビット反転させたデータ
                    let error = (1 << i) | (1 << j) | (1 << k) | (1 << l);  // エラービット
                    println!("error: {:<024b}", error);
                    let rx = encoded ^ error;  // errorでビット反転させる．
                    println!("rx: {:<024b}", rx);
                    let (flag, corrected) = ecc(rx);
                    println!("corrected: {:<024b}", corrected);

                    // 符号語から元のデータを取り出す
                    println!("tx (recover): {:<012b}, flag: {:?}", decode(corrected), flag);
                    println!("{:?}", tx == decode(corrected));
                    if error.count_ones() < 4 {
                        assert_eq!(tx, decode(corrected));
                    }
                }
            }
        }
    }
    */
}

/// 12bitのデータを24bitの符合語に変換
/// 
/// データは下位12bitに入れておく．
/// 上位4bitは見ないので何でも良い．
fn encode(a: u16) -> u32 {
    let mut c = 0;  // 符号語(code)
    // aベクトルとG行列の積（加算はXOR）
    for (i, g_val) in G.iter().enumerate() {
        let tmp = 0x800 >> i;
        let a_bit = if a & tmp == tmp {0xFFFFFF} else {0};
        c ^= a_bit & *g_val;
    }
    c
}

/// 受信後のエラー訂正を行う．
/// 
/// r: 受信語（エラーによるビット反転の可能性あり）
/// 
/// return: (flag, code)
/// 
/// flag: 誤りを訂正できたらtrue，4bit誤りを検出したらfalseを返す．
/// 5bit以上のエラーではtrueを返す場合もあるが，正しく訂正できているわけではない．
/// 
/// code: 誤り訂正した符号語．4bit誤りの場合は受信語をそのまま返す．
fn ecc(r: u32) -> (bool, u32) {
    // 1つめのシンドローム
    let mut s = 0;
    // rベクトルとH_T行列の積（加算はXOR）
    for (i, h_t_val) in H_T.iter().enumerate() {
        let tmp = 0x800000 >> i;
        let e_bit = if r & tmp == tmp {0xFFF} else {0};
        s ^= e_bit & *h_t_val;
    }

    // シンドロームが0なら誤りなし（もしくは検出できない）．
    // weightの処理が少し重いのでここで返してしまう．
    if s == 0 {
        return (true, r);
    }

    if weight(s) <= 3 {
        return ( true, r ^ (s as u32) );
    } else {
        for (i, h_t_val) in H_T.iter().take(12).enumerate() {
            let tmp = s ^ *h_t_val;
            if weight(tmp) <= 2 {
                let e = (0x800000 >> i) | (tmp as u32);
                //let e = G[i] ^ s as u32;  // こう書いても同じ
                return (true, r ^ e);
            }
        }
    }

    // 2つめのシンドローム
    let mut sh = 0;
    for (i, h_t_val) in H_T.iter().take(12).enumerate() {
        let tmp = 0x800 >> i;
        let s_bit = if s & tmp == tmp {0xFFF} else {0};
        sh ^= s_bit & *h_t_val;
    }
    if weight(sh) <= 3 {
        return ( true, r ^ ((sh as u32) << 12) );
    } else {
        for (i, h_t_val) in H_T.iter().take(12).enumerate() {
            let tmp = sh ^ *h_t_val;
            if weight(tmp) <= 2 {
                let e = ((tmp as u32) << 12) | (0x800 >> i);
                return (true, r ^ e);
            }
        }
    }

    // 4bit以上反転していてもエラーが全て右12bitに集中していればデータ部は
    // 問題なく復号できてしまうので，とりあえず受信語をそのまま変えす．
    (false, r)
}

/// 符合語からデータを取り出す
fn decode(a: u32) -> u16 {
    ((a >> 12) & 0xFFF) as u16
}

/// シンドロームの重みを計算する（1になっているビットを数える）．
/// 
/// s: シンドローム（右詰め12bit）
fn weight(s: u16) -> u32 {
    /*
    let mut w = 0;
    for i in 0..12 {
        w += (s >> i) & 1;
    }
    w // u16のまま返せば良い
    */
    s.count_ones()
}