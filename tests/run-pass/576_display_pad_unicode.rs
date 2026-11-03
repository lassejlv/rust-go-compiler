// Display impls using f.pad / pad_integral / debug_struct; width counts chars (not bytes) for unicode strings
use std::fmt;

struct Name(&'static str);
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.0)
    }
}

struct Num(i64);
impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.0.unsigned_abs().to_string();
        f.pad_integral(self.0 >= 0, "", &s)
    }
}
impl fmt::LowerHex for Num {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:x}", self.0.unsigned_abs());
        f.pad_integral(self.0 >= 0, "0x", &s)
    }
}

struct Celsius(f64);
impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prec = f.precision().unwrap_or(1);
        let s = format!("{:.*}°C", prec, self.0);
        if let Some(w) = f.width() {
            write!(f, "{:>w$}", s, w = w)
        } else {
            write!(f, "{}", s)
        }
    }
}

struct Point {
    x: i32,
    y: i32,
}
impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pt").field("x", &self.x).field("y", &self.y).finish()
    }
}
struct Pair(i32, &'static str);
impl fmt::Debug for Pair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Pair").field(&self.0).field(&self.1).finish()
    }
}
struct Bag(Vec<(&'static str, u32)>);
impl fmt::Debug for Bag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
    }
}
struct Many(Vec<u8>);
impl fmt::Debug for Many {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.0.iter().take(3)).finish_non_exhaustive()
    }
}
struct Set3;
impl fmt::Debug for Set3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entry(&1).entry(&"two").entry(&3.0).finish()
    }
}

fn main() {
    for n in ["ab", "héllo", "日本語", "🎉x", "e\u{301}"] {
        println!("[{:>8}] [{:<8}] [{:^8}] [{:*^7.2}] [{}]", n, n, n, n, n.chars().count());
    }
    for n in [Name("plain"), Name("ünï"), Name("中文字")] {
        println!("[{:>7}] [{:-<7}] [{:.2}] [{:^9.1}]", n, n, n, n);
    }
    for v in [0i64, 42, -42, i64::MIN] {
        println!("[{:>6}] [{:<+6}] [{:06}] [{:x}] [{:#x}] [{:#010x}]", Num(v), Num(v), Num(v), Num(v), Num(v), Num(v));
    }
    println!("[{}] [{:.3}] [{:10}] [{:10.0}]", Celsius(21.456), Celsius(21.456), Celsius(-3.0), Celsius(99.5));
    let p = Point { x: 3, y: -4 };
    println!("{:?}", p);
    println!("{:#?}", p);
    println!("{:?} {:#?}", Pair(1, "a"), Pair(2, "b"));
    println!("{:?}", Bag(vec![("k1", 1), ("k2", 22)]));
    println!("{:#?}", Bag(vec![("z", 0)]));
    println!("{:?}", Many(vec![1, 2, 3, 4, 5]));
    println!("{:?}", Set3);
    println!("{:>10?}|{:<10?}|", Some("é"), 'ß');
    println!("[{:^5}] [{:>3}]", '✓', '日');
    println!("[{:5}] [{:<5}] [{:>5}]", true, false, "ok");
    let table = [("Name", "Ünïcode"), ("名前", "x"), ("emoji🎉", "ok")];
    for (a, b) in table {
        println!("|{:<8}|{:>8}|", a, b);
    }
    let w = 12;
    println!("[{:^w$}]", Name("center"));
    println!("[{:>1$}]", Name("r"), 4);
    println!("[{:.0}]", Name("gone"));
    println!("{}", format!("{:>5}", Num(-7)).len());
}
