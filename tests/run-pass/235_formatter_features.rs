// Formatter features used by library Display impls: pad, width, precision, alternate, sign flags, fill.
use std::fmt;

struct Name(&'static str);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // f.pad respects width / fill / alignment / precision
        f.pad(self.0)
    }
}

struct Money {
    cents: i64,
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.cents < 0 { "-" } else if f.sign_plus() { "+" } else { "" };
        let abs = self.cents.unsigned_abs();
        let mut s = format!("{}${}.{:02}", sign, abs / 100, abs % 100);
        if f.alternate() {
            s = format!("{} USD", s);
        }
        match f.width() {
            Some(w) => {
                let fill = f.fill();
                let pad = w.saturating_sub(s.chars().count());
                let p: String = std::iter::repeat(fill).take(pad).collect();
                match f.align() {
                    Some(fmt::Alignment::Left) => write!(f, "{}{}", s, p),
                    Some(fmt::Alignment::Center) => {
                        let l = pad / 2;
                        write!(f, "{}{}{}", &p[..l * fill.len_utf8()], s, &p[l * fill.len_utf8()..])
                    }
                    _ => write!(f, "{}{}", p, s),
                }
            }
            None => f.write_str(&s),
        }
    }
}

struct Vec2(f64, f64);

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.*}, {:.*})", p, self.0, p, self.1)
        } else {
            write!(f, "({}, {})", self.0, self.1)
        }
    }
}

impl fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "Vec2 {{\n    x: {:?},\n    y: {:?},\n}}", self.0, self.1)
        } else {
            write!(f, "v({:?},{:?})", self.0, self.1)
        }
    }
}

struct Hex<'a>(&'a [u8]);

impl fmt::LowerHex for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            f.write_str("0x")?;
        }
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}
impl fmt::UpperHex for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, b) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(":")?;
            }
            write!(f, "{:02X}", b)?;
        }
        Ok(())
    }
}
impl fmt::Binary for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(|b| format!("{:08b}", b)).collect();
        f.pad_integral(true, "0b", &parts.join("_"))
    }
}

struct Celsius(f64);
impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(1);
        let s = format!("{:.*}°C", prec, self.0);
        f.pad_integral(self.0 >= 0.0, "", s.trim_start_matches('-'))
    }
}

fn main() {
    let n = Name("rust");
    println!("[{}] [{:>8}] [{:<8}] [{:^8}] [{:*^9}] [{:.2}] [{:>6.3}]", n, n, n, n, n, n, n);
    let w = 7;
    println!("[{:w$}] [{:>1$}] [{:-<w$.2}]", n, 6, n);

    for c in [12345, -50, 7, 0] {
        let m = Money { cents: c };
        println!("[{}] [{:+}] [{:#}] [{:>12}] [{:<12}] [{:_^14}] [{:+#16}]", m, m, m, m, m, m, m);
    }

    let v = Vec2(1.0 / 3.0, -2.5);
    println!("{} | {:.2} | {:.0} | {:?}", v, v, v, v);
    println!("{:#?}", v);
    println!("{:?}", vec![Vec2(1.0, 2.0), Vec2(0.5, 0.0)]);

    let h = Hex(&[0xde, 0xad, 0xbe, 0xef, 0x01]);
    println!("{:x} {:#x} {:X}", h, h, h);
    println!("[{:b}] [{:#b}]", Hex(&[5, 255]), Hex(&[1]));
    println!("[{:>30b}]", Hex(&[3]));

    for t in [21.456, -3.0, 0.04] {
        let c = Celsius(t);
        println!("[{}] [{:.2}] [{:+}] [{:>10}] [{:<+10.0}] [{:010.1}]", c, c, c, c, c, c);
    }

    // builtin numeric formatting flags
    println!("[{:+05}] [{:<+5}] [{:^+7.1}] [{:#010b}] [{:#06x}] [{:08.3}]", 42, 7, 3.14159, 5u8, 255, -1.5);
    println!("[{:e}] [{:E}] [{:.2e}] [{:10.1e}]", 1234.5678, 0.00012, 123456.0, 9.99);
    println!("[{:?}] [{:5}] [{:<5}] [{:^5?}]", 'x', true, false, 'y');
}
