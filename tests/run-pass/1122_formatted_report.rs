// A formatting-heavy report generator: table layout engine with column
// alignment, unicode-aware widths, number formatting (thousands separators,
// percentages, scientific, currency, signed), nested sections, custom
// Display/Debug impls honoring width/precision/flags, and text wrapping.
use std::collections::BTreeMap;
use std::fmt::{self, Display, Write};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Align { Left, Right, Center }

#[derive(Clone, Debug)]
enum Cell {
    Text(String),
    Int(i64),
    Money(i64),
    Pct(f64),
    Float(f64, usize),
    Sci(f64),
    Empty,
}

fn thousands(n: i64) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    if n < 0 { out.push('-'); }
    out.chars().rev().collect()
}

impl Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Cell::Text(t) => t.clone(),
            Cell::Int(n) => thousands(*n),
            Cell::Money(c) => {
                let sign = if *c < 0 { "-" } else { "" };
                format!("{}${}.{:02}", sign, thousands(c.abs() / 100), c.abs() % 100)
            }
            Cell::Pct(p) => format!("{:+.1}%", p * 100.0),
            Cell::Float(x, prec) => format!("{:.*}", prec, x),
            Cell::Sci(x) => format!("{:.3e}", x),
            Cell::Empty => "—".to_string(),
        };
        f.pad(&s)
    }
}

fn width(s: &str) -> usize {
    // treat CJK as width 2, combining marks as 0
    s.chars().map(|c| match c as u32 {
        0x0300..=0x036F => 0,
        0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFF00..=0xFF60 => 2,
        _ => 1,
    }).sum()
}

fn pad(s: &str, w: usize, a: Align) -> String {
    let cur = width(s);
    if cur >= w { return s.to_string(); }
    let extra = w - cur;
    match a {
        Align::Left => format!("{}{}", s, " ".repeat(extra)),
        Align::Right => format!("{}{}", " ".repeat(extra), s),
        Align::Center => format!("{}{}{}", " ".repeat(extra / 2), s, " ".repeat(extra - extra / 2)),
    }
}

struct Table {
    title: String,
    headers: Vec<(String, Align)>,
    rows: Vec<Vec<Cell>>,
    footer: Option<Vec<Cell>>,
}

impl Table {
    fn render(&self, style: &Style) -> String {
        let n = self.headers.len();
        let mut cols: Vec<usize> = self.headers.iter().map(|(h, _)| width(h)).collect();
        let fmt_rows: Vec<Vec<String>> = self.rows.iter().chain(self.footer.iter()).map(|r| r.iter().map(|c| c.to_string()).collect()).collect();
        for r in &fmt_rows {
            for (i, c) in r.iter().enumerate().take(n) {
                cols[i] = cols[i].max(width(c));
            }
        }
        let total: usize = cols.iter().sum::<usize>() + 3 * n + 1;
        let mut out = String::new();
        let line = |l: char, m: char, r: char| -> String {
            let mut s = String::new();
            s.push(l);
            for (i, w) in cols.iter().enumerate() {
                s.push_str(&style.h.to_string().repeat(w + 2));
                s.push(if i + 1 == n { r } else { m });
            }
            s
        };
        writeln!(out, "{}", pad(&self.title, total, Align::Center).trim_end()).unwrap();
        writeln!(out, "{}", line(style.tl, style.tm, style.tr)).unwrap();
        let hdr: Vec<String> = self.headers.iter().zip(&cols).map(|((h, _), w)| pad(h, *w, Align::Center)).collect();
        writeln!(out, "{v} {} {v}", hdr.join(&format!(" {} ", style.v)), v = style.v).unwrap();
        writeln!(out, "{}", line(style.ml, style.mm, style.mr)).unwrap();
        for (ri, r) in fmt_rows.iter().enumerate() {
            if ri == self.rows.len() {
                writeln!(out, "{}", line(style.ml, style.mm, style.mr)).unwrap();
            }
            let cells: Vec<String> = (0..n).map(|i| pad(r.get(i).map(|s| s.as_str()).unwrap_or(""), cols[i], self.headers[i].1)).collect();
            writeln!(out, "{v} {} {v}", cells.join(&format!(" {} ", style.v)), v = style.v).unwrap();
        }
        writeln!(out, "{}", line(style.bl, style.bm, style.br)).unwrap();
        out
    }
}

struct Style { h: char, v: char, tl: char, tm: char, tr: char, ml: char, mm: char, mr: char, bl: char, bm: char, br: char }

const ASCII: Style = Style { h: '-', v: '|', tl: '+', tm: '+', tr: '+', ml: '+', mm: '+', mr: '+', bl: '+', bm: '+', br: '+' };
const BOX: Style = Style { h: '─', v: '│', tl: '┌', tm: '┬', tr: '┐', ml: '├', mm: '┼', mr: '┤', bl: '└', bm: '┴', br: '┘' };

fn wrap(text: &str, w: usize, indent: &str) -> Vec<String> {
    let mut lines = vec![];
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if !cur.is_empty() && width(&cur) + 1 + width(word) > w {
            lines.push(format!("{}{}", indent, cur));
            cur.clear();
        }
        if !cur.is_empty() { cur.push(' '); }
        cur.push_str(word);
    }
    if !cur.is_empty() { lines.push(format!("{}{}", indent, cur)); }
    lines
}

struct Temperature(f64);
impl Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(1);
        let s = if f.alternate() { format!("{:.*}°F", prec, self.0 * 9.0 / 5.0 + 32.0) } else { format!("{:.*}°C", prec, self.0) };
        if f.sign_plus() && self.0 >= 0.0 { f.pad(&format!("+{}", s)) } else { f.pad(&s) }
    }
}

#[derive(Debug, Clone, Default)]
struct Region { name: String, q: [i64; 4], target: i64, notes: Option<String> }

fn main() {
    let regions = vec![
        Region { name: "North America".into(), q: [1_204_500_00, 1_350_220_00, 990_000_00, 1_610_750_25], target: 5_000_000_00, notes: Some("Strong Q4 driven by holiday promotions and a new partner channel.".into()) },
        Region { name: "Europe".into(), q: [880_100_10, 910_000_00, 870_500_50, 1_020_000_00], target: 3_500_000_00, notes: None },
        Region { name: "日本".into(), q: [450_000_00, 470_250_00, 300_000_00, 510_000_99], target: 2_000_000_00, notes: Some("Typhoon disruption in Q3; recovered.".into()) },
        Region { name: "Café Société".into(), q: [10_000_00, -2_500_00, 12_000_00, 0], target: 50_000_00, notes: Some("Refund in Q2.".into()) },
        Region { name: "Señor Ñandú".into(), q: [0, 0, 0, 1], target: 1, notes: None },
    ];
    let mut t = Table {
        title: "Quarterly Revenue (FY2024)".into(),
        headers: vec![
            ("Region".into(), Align::Left), ("Q1".into(), Align::Right), ("Q2".into(), Align::Right),
            ("Q3".into(), Align::Right), ("Q4".into(), Align::Right), ("Total".into(), Align::Right),
            ("vs Target".into(), Align::Right), ("Trend".into(), Align::Center),
        ],
        rows: vec![],
        footer: None,
    };
    let mut col_totals = [0i64; 5];
    for r in &regions {
        let total: i64 = r.q.iter().sum();
        let trend: String = r.q.windows(2).map(|w| match w[1].cmp(&w[0]) { std::cmp::Ordering::Greater => '↑', std::cmp::Ordering::Less => '↓', _ => '→' }).collect();
        let mut row = vec![Cell::Text(r.name.clone())];
        row.extend(r.q.iter().map(|&x| if x == 0 { Cell::Empty } else { Cell::Money(x) }));
        row.push(Cell::Money(total));
        row.push(Cell::Pct(total as f64 / r.target as f64 - 1.0));
        row.push(Cell::Text(trend));
        for (i, v) in r.q.iter().chain(std::iter::once(&total)).enumerate() { col_totals[i] += v; }
        t.rows.push(row);
    }
    let mut footer = vec![Cell::Text("TOTAL".into())];
    footer.extend(col_totals.iter().map(|&x| Cell::Money(x)));
    footer.push(Cell::Empty);
    footer.push(Cell::Text(String::new()));
    t.footer = Some(footer);
    print!("{}", t.render(&ASCII));
    print!("{}", t.render(&BOX));

    println!("\nNotes:");
    for r in regions.iter().filter(|r| r.notes.is_some()) {
        let lines = wrap(r.notes.as_deref().unwrap(), 30, "    ");
        println!("  * {}:", r.name);
        for l in lines { println!("{}", l); }
    }

    println!("\nNumber formatting gallery:");
    let nums = [0.0f64, 1.0, -1.5, 123.456789, 1e-7, 6.02214076e23, f64::MAX, f64::MIN_POSITIVE, 0.1 + 0.2];
    for x in nums {
        println!("[{:>12.3}] [{:<12e}] [{:^+14.2e}] [{:>10.0}] [{:?}]", x, x, x, x, x);
    }
    for n in [0i64, 7, -7, 255, -65536, i64::MAX, i64::MIN] {
        println!("[{:>6}] [{:<+8}] [{:^9}] [{:#x}] [{:#010b}] [{:o}] [{:>22}]", n, n, n, n as u8, (n & 0xff) as u8, n.unsigned_abs() % 512, thousands(n));
    }
    let w = 7;
    let p = 2;
    println!("[{:w$.p$}] [{:>w$}] [{:<2$}] [{:.*}] [{:>+w2$.p2$}]", 3.14159, "ab", 5, 3, 2.71828, -1.0f64, w2 = 9, p2 = 4);
    println!("[{:*^15}] [{:->10}] [{:_<8}] [{:#>4}]", "mid", 42, 'x', "toolong");
    let name = "width";
    println!("[{name:>10}] [{name:^10}] [{0:?}] [{0:>8?}]", name);
    println!("{:?} {:?} {:?}", "tab\t\"quote\"\n", 'é', '\u{7f}');
    println!("{:#?}", regions[3]);
    println!("{:?}", (1, "two", [3.0, 4.5], Some(()), None::<u8>));

    println!("\nTemperatures:");
    for c in [-40.0, 0.0, 21.5, 100.0] {
        let tc = Temperature(c);
        println!("[{}] [{:+}] [{:#}] [{:>10.2}] [{:<+#12.0}]", tc, tc, tc, tc, tc);
    }

    println!("\nHistogram:");
    let mut buckets: BTreeMap<&str, usize> = BTreeMap::new();
    for word in "the cat and the hat and the bat sat on the mat with a cat".split(' ') {
        *buckets.entry(word).or_insert(0) += 1;
    }
    let maxw = buckets.keys().map(|k| k.len()).max().unwrap();
    let maxc = *buckets.values().max().unwrap();
    for (k, v) in &buckets {
        let bar = "█".repeat(v * 20 / maxc);
        let frac = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][(v * 20 * 8 / maxc) % 8];
        println!("{:>maxw$} │{}{} {}", k, bar, frac, v, maxw = maxw);
    }
    let mut s = String::new();
    for (i, w) in ["alpha", "beta", "γάμμα", "δ"].iter().enumerate() {
        write!(&mut s, "{}{:<6}|", if i == 0 { "|" } else { "" }, w).unwrap();
    }
    println!("{}", s);
    println!("{}", format_args!("{} + {} = {}", 2, 2, 2 + 2));
    let escaped = format!("{{literal}} {{{}}}", 5);
    println!("{} {}", escaped, escaped.len());
}
