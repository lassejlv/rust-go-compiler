// A fixed-point decimal type (rust_decimal-lite): i128 mantissa + scale, parsing, arithmetic with
// rescaling, rounding modes, Display with scale, Ord, Sum, and checked ops.
use std::cmp::Ordering;
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
struct Decimal {
    m: i128,
    scale: u32,
}

#[derive(Debug, PartialEq)]
enum DecError {
    Empty,
    BadChar(char),
    TooPrecise,
    DivByZero,
    Overflow,
}

#[derive(Clone, Copy, Debug)]
enum Rounding {
    HalfUp,
    HalfEven,
    Down,
    Ceil,
}

const MAX_SCALE: u32 = 18;

fn pow10(n: u32) -> i128 {
    10i128.pow(n)
}

impl Decimal {
    fn new(m: i128, scale: u32) -> Self {
        Decimal { m, scale }
    }
    fn rescale_up(&self, scale: u32) -> i128 {
        self.m * pow10(scale - self.scale)
    }
    fn align(a: Decimal, b: Decimal) -> (i128, i128, u32) {
        let s = a.scale.max(b.scale);
        (a.rescale_up(s), b.rescale_up(s), s)
    }
    fn normalize(mut self) -> Self {
        while self.scale > 0 && self.m % 10 == 0 {
            self.m /= 10;
            self.scale -= 1;
        }
        self
    }
    fn round_dp(self, dp: u32, mode: Rounding) -> Self {
        if dp >= self.scale {
            return self;
        }
        let div = pow10(self.scale - dp);
        let q = self.m / div;
        let r = self.m % div;
        let half = div / 2;
        let ar = r.abs();
        let sign = if self.m < 0 { -1 } else { 1 };
        let bump = match mode {
            Rounding::Down => false,
            Rounding::HalfUp => ar >= half,
            Rounding::HalfEven => ar > half || (ar == half && q % 2 != 0),
            Rounding::Ceil => r > 0,
        };
        let q = if bump { q + if matches!(mode, Rounding::Ceil) { 1 } else { sign } } else { q };
        Decimal::new(q, dp)
    }
    fn checked_mul(self, o: Decimal) -> Result<Decimal, DecError> {
        let m = self.m.checked_mul(o.m).ok_or(DecError::Overflow)?;
        let mut d = Decimal::new(m, self.scale + o.scale);
        if d.scale > MAX_SCALE {
            d = d.round_dp(MAX_SCALE, Rounding::HalfEven);
        }
        Ok(d)
    }
    fn checked_div(self, o: Decimal) -> Result<Decimal, DecError> {
        if o.m == 0 {
            return Err(DecError::DivByZero);
        }
        // compute with MAX_SCALE + 1 digits then round
        let target = MAX_SCALE + 1;
        let (a, b, _) = Decimal::align(self, o);
        let num = a.checked_mul(pow10(target)).ok_or(DecError::Overflow)?;
        Ok(Decimal::new(num / b, target).round_dp(MAX_SCALE, Rounding::HalfEven).normalize())
    }
}

impl FromStr for Decimal {
    type Err = DecError;
    fn from_str(s: &str) -> Result<Self, DecError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(DecError::Empty);
        }
        let (neg, body) = match s.as_bytes()[0] {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        let mut m: i128 = 0;
        let mut scale = 0;
        let mut seen_dot = false;
        for c in body.chars() {
            match c {
                '0'..='9' => {
                    m = m * 10 + c.to_digit(10).unwrap() as i128;
                    if seen_dot {
                        scale += 1;
                    }
                }
                '.' if !seen_dot => seen_dot = true,
                '_' => {}
                other => return Err(DecError::BadChar(other)),
            }
        }
        if scale > MAX_SCALE {
            return Err(DecError::TooPrecise);
        }
        Ok(Decimal::new(if neg { -m } else { m }, scale))
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d = match f.precision() {
            Some(p) => self.round_dp(p as u32, Rounding::HalfUp),
            None => *self,
        };
        let neg = d.m < 0;
        let digits = d.m.unsigned_abs().to_string();
        let s = if d.scale == 0 {
            digits
        } else {
            let sc = d.scale as usize;
            let padded = format!("{:0>width$}", digits, width = sc + 1);
            let (i, frac) = padded.split_at(padded.len() - sc);
            format!("{}.{}", i, frac)
        };
        let mut s = if neg { format!("-{}", s) } else { s };
        if let Some(p) = f.precision() {
            if d.scale < p as u32 {
                if d.scale == 0 && p > 0 {
                    s.push('.');
                }
                for _ in d.scale..p as u32 {
                    s.push('0');
                }
            }
        }
        f.pad_integral(true, "", &s)
    }
}

impl PartialEq for Decimal {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Decimal {}
impl PartialOrd for Decimal {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Decimal {
    fn cmp(&self, o: &Self) -> Ordering {
        let (a, b, _) = Decimal::align(*self, *o);
        a.cmp(&b)
    }
}

impl Add for Decimal {
    type Output = Decimal;
    fn add(self, o: Decimal) -> Decimal {
        let (a, b, s) = Decimal::align(self, o);
        Decimal::new(a + b, s)
    }
}
impl Sub for Decimal {
    type Output = Decimal;
    fn sub(self, o: Decimal) -> Decimal {
        self + (-o)
    }
}
impl Neg for Decimal {
    type Output = Decimal;
    fn neg(self) -> Decimal {
        Decimal::new(-self.m, self.scale)
    }
}
impl Mul for Decimal {
    type Output = Decimal;
    fn mul(self, o: Decimal) -> Decimal {
        self.checked_mul(o).expect("overflow")
    }
}
impl Div for Decimal {
    type Output = Decimal;
    fn div(self, o: Decimal) -> Decimal {
        self.checked_div(o).expect("div error")
    }
}
impl Sum for Decimal {
    fn sum<I: Iterator<Item = Decimal>>(it: I) -> Decimal {
        it.fold(Decimal::new(0, 0), |a, b| a + b)
    }
}

fn d(s: &str) -> Decimal {
    s.parse().unwrap()
}

fn main() {
    for s in ["0", "1.50", "-0.007", "+12_345.6789", "", "1.2.3", "12a", "0.1234567890123456789"] {
        println!("{:<24} -> {:?}", format!("{:?}", s), s.parse::<Decimal>().map(|x| x.to_string()));
    }
    let a = d("0.1");
    let b = d("0.2");
    println!("0.1 + 0.2 = {} (== 0.3: {})", a + b, a + b == d("0.3"));
    println!("1.5 - 2.75 = {}", d("1.5") - d("2.75"));
    println!("1.25 * -3.2 = {}", d("1.25") * d("-3.2"));
    println!("10 / 3 = {}", d("10") / d("3"));
    println!("1 / 8 = {}", d("1") / d("8"));
    println!("-7.5 / 2.5 = {}", d("-7.5") / d("2.5"));
    println!("1 / 0 = {:?}", d("1").checked_div(d("0")));
    let big = Decimal::new(i128::MAX / 2, 0);
    println!("overflow: {:?}", big.checked_mul(d("3")));

    let x = d("2.345");
    let y = d("-2.345");
    for mode in [Rounding::HalfUp, Rounding::HalfEven, Rounding::Down, Rounding::Ceil] {
        println!(
            "{:<9} {} {} {} {}",
            format!("{:?}", mode),
            x.round_dp(2, mode),
            y.round_dp(2, mode),
            d("2.335").round_dp(2, mode),
            d("0.5").round_dp(0, mode)
        );
    }
    println!("[{:.2}] [{:.0}] [{:>10.3}] [{:<8}] [{:+}] [{:.4}]", d("3.14159"), d("2.5"), d("-1.5"), d("7.1"), d("4.20"), d("12"));
    let prices: Vec<Decimal> = ["19.99", "5.01", "0.10", "100"].iter().map(|s| d(s)).collect();
    let total: Decimal = prices.iter().copied().sum();
    let tax = (total * d("0.0825")).round_dp(2, Rounding::HalfUp);
    println!("total={} tax={} grand={}", total, tax, total + tax);
    let mut sorted = prices.clone();
    sorted.push(d("5.010"));
    sorted.sort();
    println!("{:?}", sorted.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    println!("normalize {} -> {}", d("1.2300"), d("1.2300").normalize());
    println!("max {}", prices.iter().max().unwrap());
}
