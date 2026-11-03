// parse errors: Display and Debug of ParseIntError, ParseFloatError, ParseBoolError, ParseCharError, IntErrorKind
use std::num::IntErrorKind;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct Rgb(u8, u8, u8);

#[derive(Debug, PartialEq)]
enum RgbError {
    BadLen(usize),
    NoHash,
    Hex(std::num::ParseIntError),
}
impl std::fmt::Display for RgbError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RgbError::BadLen(n) => write!(f, "expected 7 chars, got {}", n),
            RgbError::NoHash => f.write_str("missing '#'"),
            RgbError::Hex(e) => write!(f, "bad hex: {}", e),
        }
    }
}
impl From<std::num::ParseIntError> for RgbError {
    fn from(e: std::num::ParseIntError) -> Self {
        RgbError::Hex(e)
    }
}
impl FromStr for Rgb {
    type Err = RgbError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 7 {
            return Err(RgbError::BadLen(s.len()));
        }
        let h = s.strip_prefix('#').ok_or(RgbError::NoHash)?;
        let r = u8::from_str_radix(&h[0..2], 16)?;
        let g = u8::from_str_radix(&h[2..4], 16)?;
        let b = u8::from_str_radix(&h[4..6], 16)?;
        Ok(Rgb(r, g, b))
    }
}

fn kind_name(k: &IntErrorKind) -> &'static str {
    match k {
        IntErrorKind::Empty => "Empty",
        IntErrorKind::InvalidDigit => "InvalidDigit",
        IntErrorKind::PosOverflow => "PosOverflow",
        IntErrorKind::NegOverflow => "NegOverflow",
        IntErrorKind::Zero => "Zero",
        _ => "other",
    }
}

fn main() {
    for s in ["42", "-17", "", "-", "+", "+5", "12a", "256", "-129", " 1", "0x10", "１"] {
        match s.parse::<i8>() {
            Ok(v) => println!("{:?} -> Ok({})", s, v),
            Err(e) => println!("{:?} -> Err({}) kind={} dbg={:?}", s, e, kind_name(e.kind()), e),
        }
    }
    for s in ["255", "256", "-0", "-1", "00000000000000000000123"] {
        println!("{:?} u8 -> {:?}", s, s.parse::<u8>().map_err(|e| e.to_string()));
    }
    println!("{:?}", "99999999999999999999".parse::<u64>().map_err(|e| e.to_string()));
    println!("{:?}", "18446744073709551615".parse::<u64>());
    println!("{:?}", "-170141183460469231731687303715884105728".parse::<i128>());
    println!("{:?}", "0".parse::<std::num::NonZeroU32>().map_err(|e| kind_name(e.kind()).to_string()));
    for s in ["3.14", "-0.0", "1e10", "1E-3", "inf", "-infinity", "NaN", "nan", ".5", "5.", "1_000.0", "abc", "", "1.2.3", "+.e1"] {
        match s.parse::<f64>() {
            Ok(v) => println!("{:?} -> {}", s, v),
            Err(e) => println!("{:?} -> Err({}) {:?}", s, e, e),
        }
    }
    for s in ["true", "false", "True", "1", ""] {
        println!("{:?} -> {:?}", s, s.parse::<bool>().map_err(|e| e.to_string()));
    }
    for s in ["a", "ab", "", "é"] {
        println!("{:?} -> {:?}", s, s.parse::<char>().map_err(|e| e.to_string()));
    }
    for s in ["#ff8000", "ff8000", "#ff80", "#gg0000", "#00ff7f"] {
        match s.parse::<Rgb>() {
            Ok(c) => println!("{} -> {:?}", s, c),
            Err(e) => println!("{} -> error: {} / {:?}", s, e, e),
        }
    }
    println!("{:?}", i32::from_str_radix("-zz", 36));
    println!("{:?}", u32::from_str_radix("1012", 2).map_err(|e| e.to_string()));
    println!("{:?}", i64::from_str_radix("7fffffffffffffff", 16));
    println!("{:?}", i64::from_str_radix("8000000000000000", 16).map_err(|e| e.to_string()));
    let total: Result<i32, _> = "1 2 3 x 5".split(' ').map(i32::from_str).sum();
    println!("{:?}", total.map_err(|e| e.to_string()));
    let addr: Result<std::net::Ipv4Addr, _> = "192.168.1.300".parse();
    println!("{:?}", addr.map_err(|e| e.to_string()));
    let addr2: std::net::Ipv4Addr = "10.0.0.1".parse().unwrap();
    println!("{} {:?}", addr2, addr2.octets());
    let e = "x".parse::<u16>().unwrap_err();
    let boxed: Box<dyn std::error::Error> = Box::new(e.clone());
    println!("{} | {:?}", boxed, boxed);
    println!("{}", e == "y".parse::<u16>().unwrap_err());
    let fe = "q".parse::<f32>().unwrap_err();
    println!("{}", fe);
    let pe: Result<(), String> = Err("x".parse::<i32>().unwrap_err()).map_err(|e| format!("wrapped: {e}"));
    println!("{:?}", pe);
}
