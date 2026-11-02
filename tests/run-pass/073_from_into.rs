use std::convert::TryFrom;

#[derive(Debug)]
struct Celsius(f64);
#[derive(Debug)]
struct Fahrenheit(f64);

impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self {
        Fahrenheit(c.0 * 9.0 / 5.0 + 32.0)
    }
}

#[derive(Debug)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl From<u32> for Rgb {
    fn from(hex: u32) -> Self {
        Rgb { r: ((hex >> 16) & 0xff) as u8, g: ((hex >> 8) & 0xff) as u8, b: (hex & 0xff) as u8 }
    }
}

impl From<Rgb> for u32 {
    fn from(c: Rgb) -> u32 {
        ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
    }
}

#[derive(Debug)]
struct Even(u32);

impl TryFrom<u32> for Even {
    type Error = String;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        if v % 2 == 0 {
            Ok(Even(v))
        } else {
            Err(format!("{} is odd", v))
        }
    }
}

#[derive(Debug)]
enum Value {
    Int(i64),
    Text(String),
    Flag(bool),
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Flag(v)
    }
}

fn describe<T: Into<Value>>(x: T) -> String {
    let v: Value = x.into();
    format!("{:?}", v)
}

fn main() {
    let f: Fahrenheit = Celsius(100.0).into();
    println!("{:?}", f);
    println!("{:?}", Fahrenheit::from(Celsius(-40.0)));
    let c = Rgb::from(0x336699);
    println!("{:?}", c);
    let back: u32 = c.into();
    println!("{:x}", back);
    for n in [4u32, 7] {
        match Even::try_from(n) {
            Ok(e) => println!("ok {:?}", e),
            Err(msg) => println!("err {}", msg),
        }
    }
    let r: Result<Even, _> = 10u32.try_into();
    println!("{:?}", r);
    println!("{}", describe(42i64));
    println!("{}", describe("hi"));
    println!("{}", describe(true));
    let s: String = "abc".into();
    let n: i64 = 7i32.into();
    let fl: f64 = 3u8.into();
    println!("{} {} {}", s, n, fl);
    let big: i64 = i64::from(u32::MAX);
    println!("{}", big);
    println!("{:?}", u8::try_from(300i32).is_err());
    println!("{:?}", i8::try_from(-5i64));
    println!("{:?}", u16::try_from(-1i32).map_err(|e| e.to_string()));
    let chars: Vec<char> = String::from("xyz").chars().collect();
    let s2 = String::from_iter(chars.iter());
    println!("{}", s2);
    let v: Vec<u8> = "AB".into();
    println!("{:?}", v);
    let boxed: Box<str> = "boxed".into();
    println!("{}", boxed);
    let from_char = u32::from('A');
    println!("{}", from_char);
    println!("{:?}", char::from_u32(0x41));
    println!("{:?}", char::try_from(66u32));
}
