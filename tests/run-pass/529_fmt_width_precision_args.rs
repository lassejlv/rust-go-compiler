// format! with width/precision from args, positional/named args, fill/align, sign, zero-pad
use std::fmt;

struct Money(i64);
impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("${}.{:02}", self.0 / 100, (self.0 % 100).abs());
        f.pad(&s)
    }
}

fn table(rows: &[(&str, f64, i32)]) {
    let w = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
    for (name, val, n) in rows {
        println!("|{:<w$}|{:>10.3}|{:^7}|", name, val, n, w = w);
    }
}

fn main() {
    let width = 12;
    let prec = 3;
    let x = 3.14159265;
    println!("[{:>width$.prec$}]", x, width = width, prec = prec);
    println!("[{:<width$.prec$}]", x);
    println!("[{:^width$.prec$}]", -x);
    println!("[{:>1$}]", "hi", 6);
    println!("[{:.*}]", 2, 1.23456);
    println!("[{:>1$.2$}]", 9.87654, 10, 1);
    println!("[{0:>1$}] [{0:<1$}]", 7, 4);
    println!("[{:+}] [{:+}] [{:+.2}]", 5, -5, 0.0);
    println!("[{:08.3}] [{:+08.2}] [{:08}]", -3.14159, 2.5, -42);
    println!("[{:#010x}] [{:#06b}] [{:#o}]", 255, 5, 64);
    println!("[{:*^9}] [{:-<6}] [{:_>6}]", "mid", "l", "r");
    println!("[{:>5}] [{:<5}] [{:^5}]", 'c', true, "s");
    let name = "val";
    let v = 42;
    println!("{name}={v:>5} {v:<5}| {v:^+7}|");
    println!("{name:?} {v:#x} {v:e}");
    let w2 = 8;
    println!("[{v:>w2$}] [{x:>w2$.prec$}]");
    println!("[{:10.3e}] [{:<12e}]", 123456.789, 0.00042);
    println!("[{:.0}] [{:.0}] [{:.0}] [{:.1}]", 0.5, 1.5, 2.5, 0.05);
    println!("[{:5}] [{:<5}] [{:>5}]", Money(12345), Money(-250), Money(7));
    println!("[{:^12}] [{:*>10}]", Money(99), Money(100000));
    println!("[{:.3}]", "truncate me");
    println!("[{:>8.2}] [{:<6.10}]", "abcdef", "xy");
    println!("[{:?}] [{:8?}] [{:<8?}]", "q", 'z', Some(1));
    println!("[{:>10?}] [{:10.2?}]", (1, 2), 1.0f64);
    println!("[{:#?}]", (1, "a"));
    table(&[("alpha", 1.0, 1), ("b", -22.125, 300), ("gamma", 1234.5678, -7)]);
    let items = ["a", "bb", "ccc"];
    for (i, it) in items.iter().enumerate() {
        println!("{:>width$}{}", "", it, width = i * 2);
    }
    println!("{0} {1} {0} {}", "a", "b");
    println!("{{literal}} {}%", 50);
    println!("[{:5}] [{:<05}] [{:^05}]", -1, 7, 3);
    println!("[{:e}] [{:E}] [{:e}]", 1500, 0.000123f32, 1u64 << 40);
    let s = format!("{:>w$}", "r", w = 3) + &format!("{:-^w$}", "", w = 4);
    println!("{}", s);
    let dynw: usize = "hello".len() * 2;
    println!("[{:>dynw$}] [{:.dynw$}]", "hi", 1.0 / 3.0);
    let mut out = String::new();
    use std::fmt::Write;
    write!(out, "{:>4}", 1).unwrap();
    writeln!(out, "|{:<4}|", 2).unwrap();
    write!(&mut out, "{:03}", 9).unwrap();
    print!("{}", out);
    println!();
    println!("{}", format_args!("{}-{}", 1, 2));
    println!("{:?} {:?} {:?} {:?}", .., ..3, 2..=4, 'a'..'c');
    eprintln!("this goes to stderr {}", 1);
    let rows = [("id", 7usize), ("name", 12), ("x", 3)];
    let header: Vec<String> = rows.iter().map(|(h, w)| format!("{:^w$}", h, w = *w)).collect();
    println!("|{}|", header.join("|"));
    let sep: Vec<String> = rows.iter().map(|(_, w)| "-".repeat(*w)).collect();
    println!("|{}|", sep.join("+"));
    for (i, name) in ["ann", "bartholomew-long", "c"].iter().enumerate() {
        println!("|{:>7}|{:<12.12}|{:^3}|", i * 1000, name, i % 2 == 0);
    }
    let prec_list = [0usize, 1, 4, 9];
    for p in prec_list {
        print!("[{:.p$}] ", std::f64::consts::PI, p = p);
    }
    println!();
    println!("[{:>+10.3}] [{:<+10.3}] [{:^+10.3}]", 1.0f32, -1.0f32, 0.5f32);
    println!("[{:08.3}] [{:+08}] [{:#08b}]", f64::NAN, 0, 5u8);
    println!("[{:1$.2$}] [{:3$}]", 2.0, 8, 3, 4);
}
