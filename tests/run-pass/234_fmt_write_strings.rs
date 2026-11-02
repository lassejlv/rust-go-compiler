// std::fmt::Write into Strings, custom writers, and code-generation style output.
use std::fmt::{self, Write};

struct Indented<'a, W: Write> {
    inner: &'a mut W,
    level: usize,
    at_line_start: bool,
}

impl<'a, W: Write> Indented<'a, W> {
    fn new(inner: &'a mut W) -> Self {
        Indented { inner, level: 0, at_line_start: true }
    }
}

impl<W: Write> Write for Indented<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, line) in s.split('\n').enumerate() {
            if i > 0 {
                self.inner.write_char('\n')?;
                self.at_line_start = true;
            }
            if !line.is_empty() {
                if self.at_line_start {
                    for _ in 0..self.level {
                        self.inner.write_str("    ")?;
                    }
                    self.at_line_start = false;
                }
                self.inner.write_str(line)?;
            }
        }
        Ok(())
    }
}

// A writer that counts and limits output
struct Limited {
    buf: String,
    limit: usize,
}

impl Write for Limited {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.buf.len() + s.len() > self.limit {
            let room = self.limit - self.buf.len();
            let mut cut = room;
            while !s.is_char_boundary(cut) {
                cut -= 1;
            }
            self.buf.push_str(&s[..cut]);
            return Err(fmt::Error);
        }
        self.buf.push_str(s);
        Ok(())
    }
}

#[derive(Debug)]
struct Field {
    name: &'static str,
    ty: &'static str,
}

fn gen_struct(out: &mut impl Write, name: &str, fields: &[Field]) -> fmt::Result {
    let mut w = Indented::new(out);
    writeln!(w, "#[derive(Debug, Clone)]")?;
    writeln!(w, "pub struct {} {{", name)?;
    w.level += 1;
    for f in fields {
        writeln!(w, "pub {}: {},", f.name, f.ty)?;
    }
    w.level -= 1;
    writeln!(w, "}}\n")?;
    writeln!(w, "impl {} {{", name)?;
    w.level += 1;
    let params: Vec<String> = fields.iter().map(|f| format!("{}: {}", f.name, f.ty)).collect();
    writeln!(w, "pub fn new({}) -> Self {{", params.join(", "))?;
    w.level += 1;
    write!(w, "Self {{ ")?;
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            w.write_str(", ")?;
        }
        w.write_str(f.name)?;
    }
    writeln!(w, " }}")?;
    w.level -= 1;
    writeln!(w, "}}")?;
    w.level -= 1;
    write!(w, "}}")
}

fn table(rows: &[(&str, f64, i64)]) -> String {
    let mut s = String::new();
    let wname = rows.iter().map(|r| r.0.len()).max().unwrap_or(4).max(4);
    writeln!(s, "{:<w$} | {:>9} | {:>6}", "name", "value", "count", w = wname).unwrap();
    writeln!(s, "{:-<w$}-+-{:->9}-+-{:->6}", "", "", "", w = wname).unwrap();
    for (n, v, c) in rows {
        writeln!(s, "{:<w$} | {:>9.3} | {:>+6}", n, v, c, w = wname).unwrap();
    }
    let total: i64 = rows.iter().map(|r| r.2).sum();
    write!(&mut s, "{:>w$}   {:>9}   {:>6}", "total", "", total, w = wname).unwrap();
    s
}

fn main() {
    let mut out = String::new();
    gen_struct(
        &mut out,
        "Point",
        &[Field { name: "x", ty: "f32" }, Field { name: "y", ty: "f32" }, Field { name: "label", ty: "String" }],
    )
    .unwrap();
    println!("{}", out);

    println!("{}", table(&[("alpha", 1.5, 3), ("b", -22.25, -7), ("gamma-ray", 1000.0, 12)]));

    let mut lim = Limited { buf: String::new(), limit: 12 };
    let r = write!(lim, "{}-{}-{}", "abcdef", 12345, "überlong");
    println!("{:?} {:?} len={}", r.is_err(), lim.buf, lim.buf.len());

    let mut s = String::new();
    for i in 0..5 {
        write!(s, "{}{}", if i > 0 { "," } else { "" }, i * i).unwrap();
    }
    s.write_char('!').unwrap();
    println!("{}", s);

    let args = format_args!("{} + {} = {}", 2, 3, 2 + 3).to_string();
    println!("{}", args);
    let mut buf = String::with_capacity(64);
    fmt::write(&mut buf, format_args!("[{:>5}|{:<5}|{:^5}]", 1, 2, 3)).unwrap();
    println!("{}", buf);
    let name = "inline";
    let width = 10;
    println!("[{name:>width$}] [{name:.3}] [{:#x}] [{:#o}] [{:e}]", 255, 8, 1234.5);
}
