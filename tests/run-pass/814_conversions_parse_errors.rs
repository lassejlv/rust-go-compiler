use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Rgb { r: u8, g: u8, b: u8 }

#[derive(Debug, PartialEq)]
enum ColorError { BadLength(usize), BadHex(String), NoHash }

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::BadLength(n) => write!(f, "expected 6 hex digits, got {}", n),
            ColorError::BadHex(s) => write!(f, "invalid hex '{}'", s),
            ColorError::NoHash => f.write_str("missing '#'"),
        }
    }
}
impl Error for ColorError {}

impl FromStr for Rgb {
    type Err = ColorError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex = s.strip_prefix('#').ok_or(ColorError::NoHash)?;
        if hex.len() != 6 { return Err(ColorError::BadLength(hex.len())); }
        let p = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| ColorError::BadHex(hex[i..i + 2].to_string()));
        Ok(Rgb { r: p(0)?, g: p(2)?, b: p(4)? })
    }
}
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() { write!(f, "rgb({}, {}, {})", self.r, self.g, self.b) } else { write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b) }
    }
}
impl From<(u8, u8, u8)> for Rgb { fn from((r, g, b): (u8, u8, u8)) -> Self { Rgb { r, g, b } } }
impl From<Rgb> for u32 { fn from(c: Rgb) -> u32 { (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32 } }
impl TryFrom<u32> for Rgb {
    type Error = String;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if v > 0xFF_FFFF { Err(format!("{:#x} out of range", v)) } else { Ok(Rgb { r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8 }) }
    }
}
impl TryFrom<&[i32]> for Rgb {
    type Error = Box<dyn Error>;
    fn try_from(v: &[i32]) -> Result<Self, Self::Error> {
        match v {
            [r, g, b] => Ok(Rgb { r: u8::try_from(*r)?, g: u8::try_from(*g)?, b: u8::try_from(*b)? }),
            _ => Err(format!("need 3 components, got {}", v.len()).into()),
        }
    }
}

#[derive(Debug)]
struct Celsius(f64);
#[derive(Debug)]
struct Fahrenheit(f64);
impl From<Celsius> for Fahrenheit { fn from(c: Celsius) -> Self { Fahrenheit(c.0 * 9.0 / 5.0 + 32.0) } }

fn parse_kv(s: &str) -> Result<(String, i64), Box<dyn Error>> {
    let (k, v) = s.split_once('=').ok_or("missing '='")?;
    let k = k.trim();
    if k.is_empty() { return Err(Box::new(ColorError::NoHash)); }
    let v: i64 = v.trim().parse()?;
    Ok((k.to_string(), v))
}

fn sum_all(items: &[&str]) -> Result<i32, ParseIntError> {
    items.iter().map(|s| s.parse::<i32>()).sum()
}

fn main() {
    for s in ["#ff8000", "#FFF", "00ff00", "#12345g", "#0a0B0c"] {
        match s.parse::<Rgb>() {
            Ok(c) => println!("{} -> {} / {:#} / {:?} / {}", s, c, c, c, u32::from(c)),
            Err(e) => println!("{} -> error: {} ({:?})", s, e, e),
        }
    }
    let c: Rgb = (10, 20, 30).into();
    let n: u32 = c.into();
    println!("{} {}", c, n);
    for v in [0x123456u32, 0x1000000] {
        let r: Result<Rgb, _> = v.try_into();
        println!("{:?}", r);
    }
    for v in [&[1, 2, 3][..], &[1, 2], &[255, 256, 0], &[-1, 0, 0]] {
        match Rgb::try_from(v) { Ok(c) => println!("ok {}", c), Err(e) => println!("err {}", e) }
    }
    println!("{:?}", Fahrenheit::from(Celsius(100.0)));
    let f: Fahrenheit = Celsius(-40.0).into();
    println!("{:?}", f);
    for s in ["a=5", "  b = -12 ", "c=x", "=3", "noeq", "d=99999999999999999999"] {
        match parse_kv(s) { Ok(kv) => println!("{:?}", kv), Err(e) => println!("'{}': {}", s, e) }
    }
    println!("{:?} {:?}", sum_all(&["1", "2", "3"]), sum_all(&["1", "zz", "3"]));
    println!("{:?}", u8::try_from(300i32).map_err(|e| e.to_string()));
    println!("{:?}", i8::try_from(-128i64));
    println!("{:?}", u16::try_from(-1i8).is_err());
    println!("{:?}", char::try_from(0x1F980u32));
    println!("{:?}", char::try_from(0xDFFFu32).map_err(|e| e.to_string()));
    println!("{:?} {:?}", u8::try_from('A'), u8::try_from('é').map_err(|e| e.to_string()));
    println!("{}", char::from(97u8));
    println!("{:?}", "".parse::<i32>().map_err(|e| e.to_string()));
    println!("{:?}", "-".parse::<i32>().map_err(|e| e.to_string()));
    println!("{:?}", "300".parse::<u8>().map_err(|e| e.to_string()));
    println!("{:?}", "-300".parse::<i8>().map_err(|e| e.to_string()));
    println!("{:?}", "+7".parse::<u8>());
    println!("{:?}", "1.5x".parse::<f32>().map_err(|e| e.to_string()));
    println!("{:?} {:?}", "true".parse::<bool>(), "yes".parse::<bool>().map_err(|e| e.to_string()));
    println!("{:?} {:?}", "x".parse::<char>(), "xy".parse::<char>().map_err(|e| e.to_string()));
    let v: Vec<u64> = "1,2,3".split(',').map(str::parse).collect::<Result<_, _>>().unwrap();
    println!("{:?}", v);
    let bad: Result<Vec<u64>, _> = "1,-2,3".split(',').map(str::parse::<u64>).collect();
    println!("{:?}", bad.map_err(|e| e.to_string()));
    let s = String::from("hello");
    let bs: Box<str> = s.clone().into_boxed_str();
    let v8: Vec<u8> = s.clone().into();
    let back = String::from_utf8(v8.clone()).unwrap();
    let from_chars = String::from_iter(['a', 'b']);
    let arr: [u8; 3] = <[u8; 3]>::try_from(&v8[1..4]).unwrap();
    let arr_err = <[u8; 2]>::try_from(&v8[..]).is_err();
    println!("{} {:?} {} {} {:?} {}", bs, v8, back, from_chars, arr, arr_err);
    let big: i64 = i64::from(7u32) + i64::from(true) + i64::from(b'x');
    let fl: f64 = f64::from(1.5f32) + f64::from(3u8);
    println!("{} {}", big, fl);
    let opt: Option<Vec<i32>> = vec![Some(1), Some(2)].into_iter().collect();
    let opt2: Option<Vec<i32>> = vec![Some(1), None].into_iter().collect();
    println!("{:?} {:?}", opt, opt2);
    let e: Box<dyn Error + Send + Sync> = From::from("plain message");
    println!("{} / {:?}", e, e.to_string());
}
