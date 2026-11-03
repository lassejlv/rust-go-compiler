// bit manipulation puzzles: n-queens with bitmasks, subset sums, sudoku bitsets, single-number xor, popcount table
fn queens(n: u32) -> (u64, Vec<usize>) {
    fn go(n: u32, row: u32, cols: u32, d1: u32, d2: u32, cur: &mut Vec<usize>, first: &mut Option<Vec<usize>>) -> u64 {
        if row == n {
            if first.is_none() {
                *first = Some(cur.clone());
            }
            return 1;
        }
        let full = (1u32 << n) - 1;
        let mut avail = full & !(cols | d1 | d2);
        let mut count = 0;
        while avail != 0 {
            let bit = avail & avail.wrapping_neg();
            avail &= avail - 1;
            cur.push(bit.trailing_zeros() as usize);
            count += go(n, row + 1, cols | bit, (d1 | bit) << 1, (d2 | bit) >> 1, cur, first);
            cur.pop();
        }
        count
    }
    let mut first = None;
    let c = go(n, 0, 0, 0, 0, &mut vec![], &mut first);
    (c, first.unwrap_or_default())
}

fn subset_sums(v: &[i32], target: i32) -> Vec<Vec<i32>> {
    let n = v.len();
    let mut out = vec![];
    for mask in 0u32..(1 << n) {
        let s: i32 = (0..n).filter(|i| mask >> i & 1 == 1).map(|i| v[i]).sum();
        if s == target {
            out.push((0..n).filter(|i| mask & (1 << i) != 0).map(|i| v[i]).collect());
        }
    }
    out
}

fn solve_sudoku(g: &mut [[u8; 9]; 9]) -> bool {
    let mut rows = [0u16; 9];
    let mut cols = [0u16; 9];
    let mut boxes = [0u16; 9];
    for r in 0..9 {
        for c in 0..9 {
            if g[r][c] != 0 {
                let b = 1 << g[r][c];
                rows[r] |= b;
                cols[c] |= b;
                boxes[r / 3 * 3 + c / 3] |= b;
            }
        }
    }
    fn rec(g: &mut [[u8; 9]; 9], rows: &mut [u16; 9], cols: &mut [u16; 9], boxes: &mut [u16; 9]) -> bool {
        let mut best: Option<(usize, usize, u16)> = None;
        for r in 0..9 {
            for c in 0..9 {
                if g[r][c] == 0 {
                    let used = rows[r] | cols[c] | boxes[r / 3 * 3 + c / 3];
                    let cand = !used & 0b11_1111_1110;
                    if best.map_or(true, |(_, _, bc)| cand.count_ones() < bc.count_ones()) {
                        best = Some((r, c, cand));
                    }
                }
            }
        }
        let Some((r, c, mut cand)) = best else { return true };
        while cand != 0 {
            let d = cand.trailing_zeros() as u8;
            cand &= cand - 1;
            let b = 1u16 << d;
            g[r][c] = d;
            rows[r] |= b;
            cols[c] |= b;
            boxes[r / 3 * 3 + c / 3] |= b;
            if rec(g, rows, cols, boxes) {
                return true;
            }
            rows[r] &= !b;
            cols[c] &= !b;
            boxes[r / 3 * 3 + c / 3] &= !b;
        }
        g[r][c] = 0;
        false
    }
    rec(g, &mut rows, &mut cols, &mut boxes)
}

fn main() {
    for n in [4, 6, 8, 10] {
        let (c, first) = queens(n);
        println!("{}-queens: {} first={:?}", n, c, first);
    }
    println!("{:?}", subset_sums(&[3, 34, 4, 12, 5, 2], 9));
    println!("{}", [4, 1, 2, 1, 2, 7, 4].iter().fold(0, |a, b| a ^ b));
    let nums = [2, 2, 3, 2, 7, 7, 7];
    let (mut ones, mut twos) = (0i32, 0i32);
    for &x in &nums {
        ones = (ones ^ x) & !twos;
        twos = (twos ^ x) & !ones;
    }
    println!("single-of-three: {}", ones);
    let table: Vec<u8> = (0..16u8).map(|i| i.count_ones() as u8).collect();
    let pc = |x: u32| -> u32 { (0..8).map(|k| table[((x >> (4 * k)) & 0xf) as usize] as u32).sum() };
    println!("{} {}", pc(0xdeadbeef), 0xdeadbeefu32.count_ones());
    let mut g: [[u8; 9]; 9] = [
        [5, 3, 0, 0, 7, 0, 0, 0, 0],
        [6, 0, 0, 1, 9, 5, 0, 0, 0],
        [0, 9, 8, 0, 0, 0, 0, 6, 0],
        [8, 0, 0, 0, 6, 0, 0, 0, 3],
        [4, 0, 0, 8, 0, 3, 0, 0, 1],
        [7, 0, 0, 0, 2, 0, 0, 0, 6],
        [0, 6, 0, 0, 0, 0, 2, 8, 0],
        [0, 0, 0, 4, 1, 9, 0, 0, 5],
        [0, 0, 0, 0, 8, 0, 0, 7, 9],
    ];
    println!("solved: {}", solve_sudoku(&mut g));
    for row in &g {
        println!("{}", row.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(""));
    }
    let swap = |mut a: u32, mut b: u32| {
        a ^= b;
        b ^= a;
        a ^= b;
        (a, b)
    };
    println!("{:?}", swap(12, 34));
    let next_perm = |v: u32| {
        let t = v | (v - 1);
        (t + 1) | (((!t & (!t).wrapping_neg()) - 1) >> (v.trailing_zeros() + 1))
    };
    let mut v = 0b0111u32;
    let mut seq = vec![];
    for _ in 0..6 {
        seq.push(format!("{:06b}", v));
        v = next_perm(v);
    }
    println!("{:?}", seq);
    let is_pow4 = |x: u32| x != 0 && x & (x - 1) == 0 && x & 0x5555_5555 != 0;
    println!("{:?}", [1, 4, 8, 16, 64, 100].map(is_pow4));
    let parity = |mut x: u64| {
        x ^= x >> 32;
        x ^= x >> 16;
        x ^= x >> 8;
        x ^= x >> 4;
        x ^= x >> 2;
        x ^= x >> 1;
        x & 1
    };
    println!("{} {}", parity(0b1011), parity(u64::MAX));
    let interleave = |x: u16, y: u16| -> u32 { (0..16).map(|i| (((x as u32 >> i) & 1) << (2 * i)) | (((y as u32 >> i) & 1) << (2 * i + 1))).sum() };
    println!("{:#x}", interleave(0xffff, 0));
    println!("{:#x}", interleave(0b101, 0b011));
}
