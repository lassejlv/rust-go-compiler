use std::fmt::{self, Write as FmtWrite};

struct Hex<'a>(&'a [u8]);
impl fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, b) in self.0.iter().enumerate() {
            if i > 0 && f.alternate() { f.write_char(':')?; }
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

struct Padded(&'static str);
impl fmt::Display for Padded {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.0)
    }
}

struct Num(i64);
impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.0.abs().to_string();
        f.pad_integral(self.0 >= 0, "0n", &s)
    }
}
impl fmt::Binary for Num {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { fmt::Binary::fmt(&self.0, f) }
}

struct Info;
impl fmt::Debug for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "w={:?} p={:?} fill={:?} align={} plus={} zero={} alt={}",
            f.width(), f.precision(), f.fill(),
            match f.align() { Some(fmt::Alignment::Left) => "L", Some(fmt::Alignment::Right) => "R", Some(fmt::Alignment::Center) => "C", None => "-" },
            f.sign_plus(), f.sign_aware_zero_pad(), f.alternate())
    }
}

struct Tree { name: &'static str, kids: Vec<Tree> }
impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.kids.is_empty() {
            f.debug_tuple("Leaf").field(&self.name).finish()
        } else {
            f.debug_struct("Node").field("name", &self.name).field("kids", &self.kids).finish_non_exhaustive()
        }
    }
}

struct Set(Vec<i32>);
impl fmt::Debug for Set {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.0.iter()).finish()
    }
}
struct Map(Vec<(&'static str, i32)>);
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
    }
}

fn main() {
    let name = "rust";
    let width = 12;
    println!("[{name:>width$}] [{name:*^width$}] [{0:-<1$}]", name, 8);
    println!("[{:>1$}] [{:.*}] [{:>w2$.p2$}]", "ab", 5, 1.0f64 / 7.0, 2.0f64.sqrt(), w2 = 9, p2 = 4);
    println!("{} / {:#}", Hex(&[0xde, 0xad, 0xbe, 0xef]), Hex(&[1, 2, 3]));
    println!("[{:>8}] [{:<8}] [{:^8}] [{:.2}] [{:>6.3}]", Padded("hi"), Padded("hi"), Padded("hi"), Padded("hello"), Padded("hello"));
    println!("[{:>6}] [{:+}] [{:06}] [{:#}] [{:<5}]", Num(-42), Num(42), Num(-7), Num(9), Num(3));
    println!("[{:#010b}] [{:b}]", Num(5), Num(-1));
    println!("{:?}", Info);
    println!("{:>+#08.3?}", Info);
    println!("{:_<5?}", Info);
    let t = Tree { name: "root", kids: vec![Tree { name: "a", kids: vec![] }, Tree { name: "b", kids: vec![Tree { name: "c", kids: vec![] }] }] };
    println!("{:?}", t);
    println!("{:#?}", t);
    println!("{:?} {:?}", Set(vec![3, 1]), Map(vec![("x", 1), ("y", 2)]));
    println!("{:#?}", Map(vec![("k", 0)]));
    let mut s = String::new();
    write!(s, "{}-{:?}", 1, "two").unwrap();
    writeln!(s, "!").unwrap();
    print!("{}", s);
    let args = format_args!("{} {}", 1, 2).to_string();
    println!("{}", args);
    println!("{{literal}} {{{}}}", 5);
    println!("{:?} {:?} {:?}", 'a', "q\"uote\n", '\'');
    println!("{:5}|{:<5}|{:>5}|{:^5}|", 'x', true, "s", 1u8);
    println!("{:?}", format!("{:>3}", "toolong"));
    let v = 255;
    println!("{v:#x} {v:#o} {v:e} {v:>+5}");
    println!("{:?}", (1, "a", [Some(1.5), None], ()));
    println!("{:#?}", (1, vec![(2, 'c')]));
    println!("{}", format!("{:?}", String::from("é\u{0}\t")));
    let p = &&&5;
    println!("{} {:p}", p, std::ptr::null::<u8>());
}
