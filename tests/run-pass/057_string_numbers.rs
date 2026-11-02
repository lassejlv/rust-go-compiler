fn to_base(mut n: u64, base: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(digits[(n % base) as usize] as char);
        n /= base;
    }
    out.iter().rev().collect()
}

fn digit_sum(n: u64) -> u64 {
    n.to_string().chars().map(|c| c.to_digit(10).unwrap() as u64).sum()
}

fn with_commas(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{}", out) } else { out }
}

fn roman(mut n: u32) -> String {
    let table = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut out = String::new();
    for &(v, s) in table.iter() {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn main() {
    for n in [0u64, 10, 255, 1000, 123456789] {
        println!("{} -> bin {} oct {} hex {} b36 {}", n, to_base(n, 2), to_base(n, 8), to_base(n, 16), to_base(n, 36));
    }
    println!("from_str_radix ff = {:?}", i64::from_str_radix("ff", 16));
    println!("from_str_radix -101 = {:?}", i32::from_str_radix("-101", 2));
    println!("from_str_radix zz = {:?}", u32::from_str_radix("zz", 36));
    println!("from_str_radix bad = {}", u8::from_str_radix("12", 2).is_err());
    println!("digit_sum(987654321) = {}", digit_sum(987654321));
    for n in [0i64, 999, 1000, 1234567, -9876543210] {
        println!("{} -> {}", n, with_commas(n));
    }
    for n in [1, 4, 9, 14, 40, 90, 400, 1994, 2024, 3999] {
        print!("{}={} ", n, roman(n));
    }
    println!();
    let s = 12345.to_string();
    let rev: String = s.chars().rev().collect();
    let back: u32 = rev.parse().unwrap();
    println!("reversed number: {}", back);
    let mut armstrong = Vec::new();
    for n in 100..1000u32 {
        let d: u32 = n.to_string().chars().map(|c| c.to_digit(10).unwrap().pow(3)).sum();
        if d == n {
            armstrong.push(n);
        }
    }
    println!("armstrong numbers: {:?}", armstrong);
    println!("char::from_digit: {:?} {:?}", char::from_digit(7, 10), char::from_digit(11, 16));
    println!("i8 parse overflow: {:?}", "200".parse::<i8>().map_err(|e| e.to_string()));
    println!("u32 parse negative: {:?}", "-1".parse::<u32>().map_err(|e| e.to_string()));
    println!("to_string on bool/char: {} {}", true.to_string(), 'q'.to_string());
}
