use std::fmt;

struct Money<T> { amount: T, cur: &'static str }
impl<T: fmt::Display> fmt::Display for Money<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{}{}", self.amount, self.cur);
        f.pad(&s)
    }
}

struct Hex<T>(T);
impl<T: fmt::LowerHex> fmt::Display for Hex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        fmt::LowerHex::fmt(&self.0, f)
    }
}

struct List<T>(Vec<T>);
impl<T: fmt::Display> fmt::Display for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            // forward the formatter options to each element
            fmt::Display::fmt(x, f)?;
        }
        write!(f, "]")
    }
}
impl<T: fmt::Debug> fmt::Debug for List<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_list().entries(self.0.iter()).finish() }
}

struct Pt<T> { x: T, y: T }
impl<T: fmt::Debug> fmt::Debug for Pt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_struct("Pt").field("x", &self.x).field("y", &self.y).finish() }
}
impl<T: fmt::Display> fmt::Display for Pt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "({:.*}, {:.*})", p, self.x, p, self.y)
        } else if f.alternate() {
            write!(f, "Pt<{} ; {}>", self.x, self.y)
        } else {
            write!(f, "({}, {})", self.x, self.y)
        }
    }
}

struct Tuple2<A, B>(A, B);
impl<A: fmt::Debug, B: fmt::Debug> fmt::Debug for Tuple2<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_tuple("T2").field(&self.0).field(&self.1).finish() }
}

fn show<T: fmt::Display + ?Sized>(t: &T) -> String { format!("<{:>12}>|<{:<12}>|<{:^12}>", t, t, t) }

fn main() {
    let m = Money { amount: 12.5, cur: "EUR" };
    println!("[{}] [{:>10}] [{:<10}] [{:*^11}]", m, m, m, m);
    println!("{}", show(&Money { amount: 3, cur: "$" }));
    println!("{} {:08x} {}", Hex(255u8), 4096, Hex(0xdeadu32));
    println!("{:8}|", Hex(17i64));
    let l = List(vec![1.5, 2.25, 3.0]);
    println!("{} {:.2} {:>6}", l, l, l);
    println!("{:?}", l);
    println!("{:#?}", List(vec![Pt { x: 1, y: 2 }]));
    let p = Pt { x: 1.23456, y: -7.0 };
    println!("{} {:.1} {:#} {:?}", p, p, p, p);
    println!("{:#?}", Pt { x: "a", y: "b" });
    println!("{:?} {:#?}", Tuple2(1, "x"), Tuple2(Some(2), Pt { x: 0u8, y: 1 }));
    println!("{}", show("plain"));
    println!("{:>5}|{:<5}|{:^5}|", 'c', true, -1);
}
