// Formatter introspection: width(), precision(), fill(), align(), sign_plus(), sign_minus(),
// alternate(), sign_aware_zero_pad(); write! into String and fmt::Write impls; format_args! passing.
use std::fmt::{self, Write};

struct Probe;
impl fmt::Display for Probe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let align = match f.align() {
            Some(fmt::Alignment::Left) => "left",
            Some(fmt::Alignment::Right) => "right",
            Some(fmt::Alignment::Center) => "center",
            None => "none",
        };
        write!(
            f,
            "w={:?} p={:?} fill={:?} align={} plus={} minus={} alt={} zero={}",
            f.width(),
            f.precision(),
            f.fill(),
            align,
            f.sign_plus(),
            f.sign_minus(),
            f.alternate(),
            f.sign_aware_zero_pad()
        )
    }
}

struct Upper<W: Write> {
    inner: W,
    count: usize,
}
impl<W: Write> Write for Upper<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.count += s.len();
        for c in s.chars() {
            for u in c.to_uppercase() {
                self.inner.write_char(u)?;
            }
        }
        Ok(())
    }
}

struct Limited {
    buf: String,
    cap: usize,
}
impl Write for Limited {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.buf.len() + s.len() > self.cap {
            return Err(fmt::Error);
        }
        self.buf.push_str(s);
        Ok(())
    }
}

fn log_args(args: fmt::Arguments) -> String {
    let mut s = String::from("LOG: ");
    s.write_fmt(args).unwrap();
    s
}

fn render_table(rows: &[(&str, f64)]) -> Result<String, fmt::Error> {
    let mut out = String::new();
    writeln!(out, "{:<8}|{:>9}", "name", "value")?;
    writeln!(out, "{:-<8}+{:->9}", "", "")?;
    for (n, v) in rows {
        writeln!(out, "{:<8}|{:>9.3}", n, v)?;
    }
    write!(out, "{:<8}|{:>9.3}", "total", rows.iter().map(|r| r.1).sum::<f64>())?;
    Ok(out)
}

fn main() {
    println!("{}", Probe);
    println!("{:>10}", Probe);
    println!("{:*^7.3}", Probe);
    println!("{:+#}", Probe);
    println!("{:-<4}", Probe);
    println!("{:08.2}", Probe);
    println!("{:_<1$}", Probe, 12);
    println!("{:.*}", 5, Probe);

    let mut s = String::new();
    write!(s, "{}-{}", 1, 2).unwrap();
    write!(&mut s, "|{:>4}|", "x").unwrap();
    s.write_char('!').unwrap();
    writeln!(s).unwrap();
    writeln!(s, "line {}", 2).unwrap();
    print!("{}", s);

    let mut up = Upper { inner: String::new(), count: 0 };
    write!(up, "hello {} straße {:?}", 42, "ß").unwrap();
    println!("{} ({} bytes in)", up.inner, up.count);

    let mut lim = Limited { buf: String::new(), cap: 10 };
    let r1 = write!(lim, "{}", "12345");
    let r2 = write!(lim, "{}", "678901");
    let r3 = write!(lim, "{}", "67890");
    println!("{:?} {:?} {:?} buf={:?}", r1.is_ok(), r2.is_ok(), r3.is_ok(), lim.buf);

    println!("{}", log_args(format_args!("{} + {} = {}", 2, 3, 2 + 3)));
    println!("{}", log_args(format_args!("{:>5}|{:<5}|", "ab", 7)));

    match render_table(&[("alpha", 1.5), ("beta", -22.25), ("gamma", 1000.0 / 3.0)]) {
        Ok(t) => println!("{}", t),
        Err(_) => println!("error"),
    }

    let parts: Vec<String> = (1..=3).map(|i| format!("{:0>3}", i * 7)).collect();
    println!("{}", parts.join(":"));
    let width = 6;
    let mut buf = String::with_capacity(4);
    for (i, w) in ["a", "bb", "ccc"].iter().enumerate() {
        let _ = write!(buf, "{:>width$}{}", w, if i < 2 { "," } else { "" });
    }
    println!("[{}]", buf);
    let dbg = format!("{:?}", format_args!("{}", "x"));
    println!("{}", dbg);
}
