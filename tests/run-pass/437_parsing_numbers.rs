// Number parsing: from_str_radix in many bases, parse error kinds, leading +/-, whitespace,
// underscores (rejected), f64 parsing edge cases (inf, nan, exponents, hex not allowed),
// char::to_digit/from_digit, and a hand-written numeric literal lexer.
use std::num::IntErrorKind;

fn kind(e: &std::num::ParseIntError) -> &'static str {
    match e.kind() {
        IntErrorKind::Empty => "Empty",
        IntErrorKind::InvalidDigit => "InvalidDigit",
        IntErrorKind::PosOverflow => "PosOverflow",
        IntErrorKind::NegOverflow => "NegOverflow",
        IntErrorKind::Zero => "Zero",
        _ => "Other",
    }
}

fn parse_literal(s: &str) -> Result<i64, String> {
    let clean: String = s.chars().filter(|&c| c != '_').collect();
    let (neg, body) = match clean.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, clean.as_str()),
    };
    let (radix, digits) = if let Some(r) = body.strip_prefix("0x") {
        (16, r)
    } else if let Some(r) = body.strip_prefix("0o") {
        (8, r)
    } else if let Some(r) = body.strip_prefix("0b") {
        (2, r)
    } else {
        (10, body)
    };
    let mag = u64::from_str_radix(digits, radix).map_err(|e| format!("{}: {}", s, kind(&e)))?;
    if neg {
        if mag == 1u64 << 63 {
            Ok(i64::MIN)
        } else {
            i64::try_from(mag).map(|v| -v).map_err(|_| format!("{}: underflow", s))
        }
    } else {
        i64::try_from(mag).map_err(|_| format!("{}: overflow", s))
    }
}

fn main() {
    let ints = ["42", "-42", "+42", " 42", "42 ", "", "-", "+", "4_2", "0042", "2147483648", "-2147483649", "-2147483648", "1e3", "٣"];
    for s in ints {
        match s.parse::<i32>() {
            Ok(v) => println!("{:?} -> Ok({})", s, v),
            Err(e) => println!("{:?} -> Err({}) {}", s, kind(&e), e),
        }
    }
    println!("{:?} {:?}", "-0".parse::<u8>(), "-1".parse::<u8>().map_err(|e| kind(&e)));
    println!("{:?} {:?}", "256".parse::<u8>().map_err(|e| kind(&e)), "-129".parse::<i8>().map_err(|e| kind(&e)));

    let radixes: [(&str, u32); 9] = [("ff", 16), ("FF", 16), ("777", 8), ("1011", 2), ("zz", 36), ("Zz", 36), ("12", 3), ("-7f", 16), ("102", 2)];
    for (s, r) in radixes {
        println!("{} base {} -> {:?}", s, r, i64::from_str_radix(s, r).map_err(|e| kind(&e)));
    }
    println!("{:?}", u32::from_str_radix("+ff", 16));
    println!("{:?}", u8::from_str_radix("100000000", 2).map_err(|e| kind(&e)));

    let floats = [
        "3.14", "-0.0", "1e10", "1E-5", ".5", "5.", "+.5e+2", "inf", "-inf", "infinity", "NaN", "nan", "1_0.0",
        "0x10", "1e400", "-1e400", "1e-400", "  1.0", "1.0.0", "e5", "9007199254740993", "2.2250738585072014e-308",
    ];
    for s in floats {
        match s.parse::<f64>() {
            Ok(v) => println!("{:?} -> {:?}", s, v),
            Err(e) => println!("{:?} -> error: {}", s, e),
        }
    }
    println!("{:?} {:?}", "3.4e38".parse::<f32>(), "3.5e38".parse::<f32>());
    println!("{:?} {:?}", "true".parse::<bool>(), "True".parse::<bool>().is_err());
    println!("{:?} {:?}", "x".parse::<char>(), "xy".parse::<char>().is_err());

    let digits: Vec<Option<u32>> = ['7', 'a', 'F', 'z', 'Z', '!'].iter().map(|c| c.to_digit(16)).collect();
    println!("{:?}", digits);
    println!("{:?} {:?}", 'z'.to_digit(36), '9'.to_digit(8));
    let made: String = (0..16).filter_map(|d| char::from_digit(d, 16)).collect();
    println!("{} {:?} {:?}", made, char::from_digit(35, 36), char::from_digit(10, 10));

    for lit in ["0xff", "-0x80", "0b1010_1010", "0o777", "1_000_000", "-9223372036854775808", "9223372036854775808", "0xg1", "0x"] {
        println!("{} => {:?}", lit, parse_literal(lit));
    }

    let csv = "10, 20,x,  30 ,,40";
    let (good, bad): (Vec<_>, Vec<_>) = csv.split(',').map(|s| s.trim().parse::<u16>()).partition(|r| r.is_ok());
    let good: Vec<u16> = good.into_iter().map(Result::unwrap).collect();
    println!("good {:?} bad {}", good, bad.len());
    let total: u32 = csv.split(',').filter_map(|s| s.trim().parse::<u32>().ok()).sum();
    println!("total {}", total);
    let hex_bytes: Result<Vec<u8>, _> = "de ad be ef".split(' ').map(|h| u8::from_str_radix(h, 16)).collect();
    println!("{:?}", hex_bytes);
}
