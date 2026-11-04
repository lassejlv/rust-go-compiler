// Breadth test of the fmt machinery: Formatter queries (width, precision,
// fill, align, flags), debug builders (struct/tuple/list/set/map) with
// alternate mode, pad/pad_integral, write!/writeln! into String and custom
// fmt::Write sinks, format_args!/Arguments, and all the formatting traits.
use std::collections::BTreeMap;
use std::fmt::{self, Binary, Debug, Display, Formatter, LowerExp, LowerHex, Octal, UpperHex, Write};

struct Probe;
impl Display for Probe {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "w={:?} p={:?} fill={:?} align={} +{} -{} #{} 0{}",
            f.width(), f.precision(), f.fill(),
            match f.align() { Some(fmt::Alignment::Left) => "L", Some(fmt::Alignment::Right) => "R", Some(fmt::Alignment::Center) => "C", None => "-" },
            f.sign_plus(), f.sign_minus(), f.alternate(), f.sign_aware_zero_pad())
    }
}

struct Bits(u16);
impl Binary for Bits { fn fmt(&self, f: &mut Formatter) -> fmt::Result { Binary::fmt(&self.0, f) } }
impl Octal for Bits { fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "o{:o}", self.0) } }
impl LowerHex for Bits { fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.pad_integral(true, "0x", &format!("{:x}", self.0)) } }
impl UpperHex for Bits { fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.pad_integral(true, "0X", &format!("{:X}", self.0)) } }
impl LowerExp for Bits { fn fmt(&self, f: &mut Formatter) -> fmt::Result { LowerExp::fmt(&(self.0 as f64), f) } }

struct Signed(i64);
impl Display for Signed {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.pad_integral(self.0 >= 0, "", &self.0.unsigned_abs().to_string()) }
}

struct Point { x: i32, y: i32 }
impl Debug for Point {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_struct("Point").field("x", &self.x).field("y", &self.y).finish() }
}
struct Pair(i32, &'static str);
impl Debug for Pair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_tuple("Pair").field(&self.0).field(&self.1).finish() }
}
struct Secret { user: String, pw: String }
impl Debug for Secret {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_struct("Secret").field("user", &self.user).finish_non_exhaustive() }
}
struct Wrapper(Vec<Point>);
impl Debug for Wrapper {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_list().entries(self.0.iter()).entry(&"end").finish() }
}
struct Registry(BTreeMap<&'static str, Vec<u8>>);
impl Debug for Registry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut m = f.debug_map();
        for (k, v) in &self.0 {
            m.key(k).value(v);
        }
        m.entry(&"extra", &Pair(1, "x")).finish()
    }
}
struct Tags(Vec<&'static str>);
impl Debug for Tags {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { f.debug_set().entries(&self.0).finish() }
}

struct Upper<'a>(&'a str);
impl Display for Upper<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // honour width/fill/alignment/precision by delegating to pad
        f.pad(&self.0.to_uppercase())
    }
}

/// fmt::Write sink that counts and wraps lines
struct Wrapping { out: String, col: usize, width: usize, writes: usize }
impl Write for Wrapping {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writes += 1;
        for c in s.chars() {
            if c == '\n' { self.col = 0; self.out.push(c); continue; }
            if self.col == self.width { self.out.push('\n'); self.col = 0; }
            self.out.push(c);
            self.col += 1;
        }
        Ok(())
    }
}

struct Failing;
impl Display for Failing {
    fn fmt(&self, _: &mut Formatter) -> fmt::Result { Err(fmt::Error) }
}

fn log_args(args: fmt::Arguments) -> String { format!("[log] {}", args) }

fn main() {
    println!("{}", Probe);
    println!("{:>8.3}", Probe);
    println!("{:*<+#12}", Probe);
    println!("{:^-010.1}", Probe);
    println!("{:_^5}", Probe);
    let b = Bits(0b1011_0010);
    println!("[{:b}] [{:#b}] [{:#018b}] [{:o}] [{:x}] [{:#x}] [{:#08X}] [{:>8x}] [{:e}]", b, b, b, b, b, b, b, b, b);
    for n in [0i64, 42, -42] {
        println!("[{}] [{:>6}] [{:<6}] [{:^6}] [{:+}] [{:06}] [{:+06}] [{:x<5}]", Signed(n), Signed(n), Signed(n), Signed(n), Signed(n), Signed(n), Signed(n), Signed(n));
    }
    let p = Point { x: 1, y: -2 };
    println!("{:?}", p);
    println!("{:#?}", p);
    println!("{:?} {:#?}", Pair(7, "seven"), Pair(0, ""));
    println!("{:?}", Secret { user: "root".into(), pw: "hunter2".into() });
    println!("{:#?}", Secret { user: "u".into(), pw: String::new() });
    let w = Wrapper(vec![Point { x: 0, y: 0 }, Point { x: 3, y: 4 }]);
    println!("{:?}", w);
    println!("{:#?}", w);
    let mut reg = BTreeMap::new();
    reg.insert("b", vec![2, 3]);
    reg.insert("a", vec![]);
    let r = Registry(reg);
    println!("{:?}", r);
    println!("{:#?}", r);
    println!("{:?} {:#?}", Tags(vec!["x", "y"]), Tags(vec![]));
    println!("{:?}", (Point { x: 9, y: 9 }, [Some(Pair(1, "a"))], "s"));
    println!("[{:>10}] [{:<10}] [{:^10}] [{:.2}] [{:-^9.3}]", Upper("abc"), Upper("ß"), Upper("mid"), Upper("truncate"), Upper("xyzzy"));
    println!("[{:>8?}] [{:<8?}] [{:^8?}] [{:08.2?}] [{:+?}]", 3, 'c', true, 1.5, 4);
    println!("[{:10?}] [{:<12?}] [{:#?}]", Some(1), "q\"t", (1,));
    println!("[{:5}] [{:<5}] [{:^5}] [{:>5}]", 'x', true, "ab", "()");
    let mut s = String::new();
    write!(s, "{}-{:03}", "id", 7).unwrap();
    writeln!(s, " ok").unwrap();
    write!(&mut s, "{:?}", vec![(1, 2)]).unwrap();
    println!("{:?}", s);
    let mut wr = Wrapping { out: String::new(), col: 0, width: 10, writes: 0 };
    write!(wr, "The quick brown {} jumps over the {} dog.\n{}", "fox", "lazy", 12345678901234u64).unwrap();
    println!("{}\n(writes: {})", wr.out, wr.writes);
    let mut sink = String::new();
    let err = write!(sink, "before {} after", Failing);
    println!("failing Display -> {:?}, partial {:?}", err, sink);
    let args_str = log_args(format_args!("{} + {} = {}", 1, 2, 1 + 2));
    println!("{}", args_str);
    let lit = format_args!("constant");
    println!("as_str {:?} {:?}", lit.as_str(), format_args!("{}", 1).as_str());
    let width = 12;
    let prec = 3;
    let val = 2.0f64.sqrt();
    println!("[{val:>width$.prec$}] [{val:<width$e}] [{:>1$}] [{:.*}]", "dyn", 6, 3.14159);
    println!("{0} {0:?} {0:>5} {1} {1:?} {name} {name:?}", "a\tb", 'c', name = "n\"m");
    println!("{{}} {{{}}} }}{{", 1);
    let v: Vec<String> = (1..=3).map(|i| format!("{:>width$}", i, width = i)).collect();
    println!("{:?}", v);
    eprintln!("this goes to stderr and is not compared");
    print!("no newline, ");
    print!("then newline\n");
    println!("{}", format!("{:?}", format!("{}", "nested")));
    println!("{:?} {:?} {:?}", 'a'..'d', ..=5, (1..2).step_by(1).len());
    println!("{:?} {:?}", std::time::Duration::from_millis(1500), std::time::Duration::from_micros(3));
    println!("{:?} {:?} {:?}", String::from("ünï"), "\u{0}\u{1b}\u{ff}", b"by\xfftes");
    println!("{:?} {:?}", std::cmp::Ordering::Less, Some(Box::new(std::rc::Rc::new(5))));
    println!("{:#?}", (vec![1, 2], (), [[0u8; 2]; 1]));
}
