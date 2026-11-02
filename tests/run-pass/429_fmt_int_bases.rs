// Integer formatting: width, fill, alignment, sign, #, 0 flags with all bases ({:x} {:X} {:o} {:b}),
// negative numbers in hex (two's complement), i128/u128, width/precision from args ({:>1$}, {:.*}),
// named args and captured identifiers.
fn hexdump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(8).enumerate() {
        out.push_str(&format!("{:04x}: ", i * 8));
        for b in chunk {
            out.push_str(&format!("{:02x} ", b));
        }
        for _ in chunk.len()..8 {
            out.push_str("   ");
        }
        out.push('|');
        for &b in chunk {
            out.push(if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        out.push_str("|\n");
    }
    out
}

fn table(rows: &[(&str, i64)]) {
    let wname = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
    println!("+{:-<w$}+{:->12}+", "", "", w = wname + 2);
    for (n, v) in rows {
        println!("| {:<w$} | {:>10} |", n, v, w = wname);
    }
    println!("+{:-<w$}+{:->12}+", "", "", w = wname + 2);
}

fn main() {
    print!("{}", hexdump(b"Hello, world!\x00\x01\xffRust"));
    table(&[("alpha", 1), ("b", -12345), ("gamma-ray", i32::MAX as i64)]);
    for shift in [0u32, 4, 8, 31] {
        let v = 1u32.wrapping_shl(shift);
        println!("{:>2}: {:#034b} {:>#12x} {:>12o}", shift, v, v, v);
    }
    let vals: [i32; 5] = [0, 7, -7, 255, i32::MIN];
    for v in vals {
        println!("[{:5}] [{:<5}] [{:^5}] [{:>+5}] [{:05}] [{:+05}]", v, v, v, v, v, v);
    }
    for v in [0u32, 1, 10, 255, 4096, u32::MAX] {
        println!("{:x} {:X} {:#x} {:#X} {:o} {:#o} {:b} {:#b}", v, v, v, v, v, v, v, v);
    }
    for v in [5u8, 200] {
        println!("[{:08b}] [{:#010b}] [{:>8x}] [{:<#6x}] [{:^#8o}] [{:*^9b}]", v, v, v, v, v, v);
    }
    let neg: i8 = -1;
    let neg16: i16 = -256;
    let neg64: i64 = -2;
    println!("{:x} {:X} {:b} {:o}", neg, neg16, neg, neg64);
    println!("{:#x} {:#b}", -128i8, i16::MIN);

    let big: u128 = u128::MAX;
    let sbig: i128 = i128::MIN;
    println!("{}", big);
    println!("{:x}", big);
    println!("{}", sbig);
    println!("{:#034x}", 0xdead_beef_u128);
    println!("{:>45}", sbig);
    println!("{:b}", 1u128 << 100);

    let w = 10;
    let p = 3;
    println!("[{:>1$}] [{:<w$}] [{:^width$}]", 42, 6, width = 8);
    println!("[{:>w$}] [{:0>w$}] [{:#0w$x}]", 99, 7, 255);
    println!("[{:.*}] [{:.p$}] [{:w$.p$}]", 2, 3.14159, 2.71828, 1.41421);
    let name = "val";
    let count = 3;
    println!("{name}={count:>4} {count:#b} {name:?}");
    println!("{0} {1} {0} {1:?} {2:x}", "a", "b", 171);

    // fill chars including unicode
    println!("[{:→<8}] [{:é>6}] [{:0^7}]", 1, 22, -3);
    // sign with zero
    println!("[{:+}] [{:+}] [{:+.1}] [{:+}]", 0, -0, -0.0f64, 0.0f32);
    // u64 and usize extremes
    println!("{} {} {}", u64::MAX, i64::MIN, usize::MAX as u64 == u64::MAX);
    println!("{:>25} {:<25}|", u64::MAX, i64::MIN);
    // Debug of integers honours hex flag
    println!("{:x?} {:X?} {:#x?}", vec![10, 255], (171u8, 4096u16), [1u8, 2]);
    // padding with Debug
    println!("[{:>6?}] [{:<8?}] [{:^10?}]", Some(1), 'x', "ab");
    // char and bool formatting
    println!("[{:>3}] [{:<6}] [{:^7}]", 'c', true, false);
    let pct = 37;
    println!("{pct}% done, {:>5.1}%", 99.95);
    println!("{:e} {:E} {:e}", 1234567u32, 0.00012f64, 100i64);
}
