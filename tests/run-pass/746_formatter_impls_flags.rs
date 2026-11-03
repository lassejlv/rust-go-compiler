// Custom Display/Debug/LowerHex/Binary impls honoring Formatter flags: width,
// precision, fill, alignment, sign_plus, alternate, zero-pad; pad() vs
// write_str, debug_* builders with alternate mode, and nested formatting.
use std::fmt::{self, Alignment, Write as _};

struct Money(i64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else if f.sign_plus() { "+" } else { "" };
        let abs = self.0.unsigned_abs();
        let prec = f.precision().unwrap_or(2).min(4);
        let scaled = match prec { 0 => format!("{}", abs / 100), p => format!("{}.{:0>width$}", abs / 100, (abs % 100) * 10u64.pow(p as u32) / 100, width = p) };
        let body = format!("{}${}", sign, scaled);
        if f.alternate() { write!(f, "[")?; }
        match (f.width(), f.align()) {
            (Some(w), Some(Alignment::Left)) => write!(f, "{:<w$}", body, w = w)?,
            (Some(w), Some(Alignment::Center)) => write!(f, "{:^w$}", body, w = w)?,
            (Some(w), _) => {
                let fill = f.fill();
                let pad = w.saturating_sub(body.chars().count());
                for _ in 0..pad { f.write_char(fill)?; }
                f.write_str(&body)?;
            }
            (None, _) => f.write_str(&body)?,
        }
        if f.alternate() { write!(f, "]")?; }
        Ok(())
    }
}

struct Name<'a>(&'a str);
impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad(self.0) }
}

struct Bits(u16);
impl fmt::Binary for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:016b}", self.0);
        let grouped: Vec<&str> = (0..4).map(|i| &s[i * 4..i * 4 + 4]).collect();
        if f.alternate() { f.pad_integral(true, "0b", &grouped.join("_")) } else { f.pad_integral(true, "", &s) }
    }
}
impl fmt::LowerHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::LowerHex::fmt(&self.0, f) }
}
impl fmt::UpperHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:X}", self.0) }
}
impl fmt::Octal for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad_integral(self.0 % 2 == 0, "0o", &format!("{:o}", self.0)) }
}
impl fmt::LowerExp for Bits {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::LowerExp::fmt(&(self.0 as f64), f) }
}

struct Matrix(Vec<Vec<f64>>);
impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(1);
        let width = f.width().unwrap_or(6);
        for row in &self.0 {
            for (i, x) in row.iter().enumerate() {
                if i > 0 { f.write_str(" ")?; }
                write!(f, "{:>w$.p$}", x, w = width, p = prec)?;
            }
            f.write_str("\n")?;
        }
        Ok(())
    }
}

struct Node { id: u32, kids: Vec<Node>, meta: Option<(&'static str, f32)> }
impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_struct("Node");
        d.field("id", &self.id);
        if let Some(m) = &self.meta { d.field("meta", m); }
        if !self.kids.is_empty() { d.field("kids", &self.kids); }
        d.finish_non_exhaustive()
    }
}
struct Pairs(Vec<(&'static str, i32)>);
impl fmt::Debug for Pairs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()?;
        f.write_str(" & ")?;
        f.debug_set().entries(self.0.iter().map(|p| p.1)).finish()?;
        f.write_str(" & ")?;
        f.debug_tuple("T").field(&self.0.len()).field(&"x").finish()?;
        f.write_str(" & ")?;
        f.debug_list().entry(&1).entries(["a", "b"]).finish()
    }
}

fn main() {
    let m = Money(-123456);
    let p = Money(990);
    println!("[{}] [{:+}] [{:.0}] [{:.3}] [{:#}] [{:>12}] [{:<12}] [{:^12}] [{:*>12}] [{:#>+12.1}]", m, p, p, m, p, m, m, p, p, p);
    println!("[{:>8}] [{:<8}] [{:^8}] [{:-^9}] [{:.2}] [{:>8.3}] [{}]", Name("bob"), Name("al"), Name("eve"), Name("x"), Name("truncate"), Name("abcdef"), Name("plain"));
    let b = Bits(0xA5F0);
    println!("[{:b}] [{:#b}] [{:#24b}] [{:x}] [{:#x}] [{:08x}] [{:#010x}] [{:X}] [{:o}] [{:#o}] [{:+o}] [{:e}] [{:.2e}]", b, b, b, b, b, b, b, b, b, b, Bits(3), b, b);
    let mx = Matrix(vec![vec![1.0, -2.5, 3.14159], vec![10.0, 0.001, -100.0]]);
    print!("{}", mx);
    print!("{:8.3}", mx);
    let tree = Node { id: 1, meta: Some(("root", 1.5)), kids: vec![Node { id: 2, kids: vec![], meta: None }, Node { id: 3, kids: vec![Node { id: 4, kids: vec![], meta: Some(("leaf", -0.0)) }], meta: None }] };
    println!("{:?}", tree);
    println!("{:#?}", tree);
    let pairs = Pairs(vec![("one", 1), ("two", 2)]);
    println!("{:?}", pairs);
    println!("{:#?}", pairs);
    let mut s = String::new();
    write!(s, "{:>5}|{:<5}|{:^5}", 1, 2, 3).unwrap();
    writeln!(s, " end").unwrap();
    write!(&mut s, "{0}-{0:?}-{name}-{name:>6}", "a", name = "nm").unwrap();
    println!("{}", s);
    let width = 7;
    let prec = 2;
    let val = 3.14159;
    println!("[{val:width$.prec$}] [{val:>+width$.1}] [{:>1$}] [{:.*}] [{:#?}]", "r", 4, 9.87, (1, "t"));
    println!("{}", format_args!("{} {}", format_args!("nested {}", 1), 2));
}
