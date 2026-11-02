// Custom Display impls that honor width/fill/alignment via f.pad and f.pad_integral,
// precision used as truncation, forwarding to inner Display with flags preserved,
// and alternate (#) mode branches.
use std::fmt;

struct Name<'a>(&'a str);
impl fmt::Display for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.0)
    }
}

struct Money {
    cents: i64,
}
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{}.{:02}", self.cents.abs() / 100, self.cents.abs() % 100);
        f.pad_integral(self.cents >= 0, "$", &s)
    }
}

struct Celsius(f64);
impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // forward precision; default 1
        let p = f.precision().unwrap_or(1);
        let s = format!("{:.*}°C", p, self.0);
        if f.alternate() {
            write!(f, "{} ({:.0}°F)", s, self.0 * 9.0 / 5.0 + 32.0)
        } else {
            f.pad(&s)
        }
    }
}

struct Hex(u32);
impl fmt::Display for Hex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // delegate to LowerHex which honors all flags
        fmt::LowerHex::fmt(&self.0, f)
    }
}
impl fmt::LowerHex for Name<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let hex: String = self.0.bytes().map(|b| format!("{:02x}", b)).collect();
        if f.alternate() {
            f.write_str("0x")?;
        }
        f.write_str(&hex)
    }
}

struct Point {
    x: f64,
    y: f64,
}
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.*}, {:.*})", p, self.x, p, self.y)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
}

struct List(Vec<i32>);
impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, v) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            // re-use caller's width for each element
            if let Some(w) = f.width() {
                write!(f, "{:>w$}", v, w = w)?;
            } else {
                write!(f, "{}", v)?;
            }
        }
        write!(f, "]")
    }
}

fn main() {
    let n = Name("Ferris");
    println!("[{}] [{:10}] [{:>10}] [{:^10}] [{:-<10}] [{:.3}] [{:>8.2}]", n, n, n, n, n, n, n);
    println!("[{:x}] [{:#x}]", n, Name("AB"));

    for c in [123456i64, -5, 0, -100000] {
        let m = Money { cents: c };
        println!("[{}] [{:>12}] [{:<12}] [{:+}] [{:012}] [{:#}] [{:^+12}]", m, m, m, m, m, m, m);
    }

    let t = Celsius(21.456);
    println!("[{}] [{:.2}] [{:#}] [{:>12}] [{:*<12.0}]", t, t, t, t, t);

    let h = Hex(48879);
    println!("[{}] [{:#}] [{:08}] [{:#010}] [{:>8}]", h, h, h, h, h);

    let p = Point { x: 1.0 / 3.0, y: -2.5 };
    println!("{} | {:.2} | {:.0}", p, p, p);

    let l = List(vec![1, 22, 333]);
    println!("{} | {:4} | {:1}", l, l, l);

    // to_string uses Display without flags
    let s = Money { cents: 999 }.to_string();
    println!("{:?} len={}", s, s.len());
    // nested format width of an already formatted value
    let inner = format!("{:>5}", Name("ab"));
    println!("[{:<9}]", inner);
    // padding with multi-byte chars counts chars not bytes
    println!("[{:>6}] [{:<6}] [{:^7}]", Name("héé"), Name("日本"), Name("ñ"));
    let names = ["a", "bbb", "cc"];
    let w = names.iter().map(|s| s.len()).max().unwrap();
    for nm in names {
        println!("|{:>w$}|{:<w$}|", Name(nm), Name(nm), w = w + 1);
    }
}
