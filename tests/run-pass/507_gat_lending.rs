// Generic associated types: lending iterators, family pattern
trait LendingIter {
    type Item<'a>
    where
        Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct WindowsMut<'t, T> {
    data: &'t mut [T],
    pos: usize,
    size: usize,
}

impl<'t, T> LendingIter for WindowsMut<'t, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a mut [T]> {
        if self.pos + self.size > self.data.len() {
            return None;
        }
        let w = &mut self.data[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

struct LineBuf {
    text: String,
    off: usize,
}
impl LendingIter for LineBuf {
    type Item<'a> = (usize, &'a str);
    fn next<'a>(&'a mut self) -> Option<(usize, &'a str)> {
        if self.off >= self.text.len() {
            return None;
        }
        let rest = &self.text[self.off..];
        let end = rest.find('\n').unwrap_or(rest.len());
        let start = self.off;
        self.off += end + 1;
        Some((start, &rest[..end]))
    }
}

trait Family {
    type Member<T>;
    fn wrap<T>(t: T) -> Self::Member<T>;
    fn name() -> &'static str;
}
struct OptionFamily;
struct VecFamily;
impl Family for OptionFamily {
    type Member<T> = Option<T>;
    fn wrap<T>(t: T) -> Option<T> {
        Some(t)
    }
    fn name() -> &'static str {
        "option"
    }
}
impl Family for VecFamily {
    type Member<T> = Vec<T>;
    fn wrap<T>(t: T) -> Vec<T> {
        vec![t]
    }
    fn name() -> &'static str {
        "vec"
    }
}

fn wrap_both<F: Family>(a: i32, b: &str) -> (F::Member<i32>, F::Member<String>) {
    (F::wrap(a), F::wrap(b.to_string()))
}

trait Store {
    type Ref<'a>: std::fmt::Debug
    where
        Self: 'a;
    fn get<'a>(&'a self, i: usize) -> Self::Ref<'a>;
}
struct Strings(Vec<String>);
impl Store for Strings {
    type Ref<'a> = Option<&'a str>;
    fn get<'a>(&'a self, i: usize) -> Option<&'a str> {
        self.0.get(i).map(|s| s.as_str())
    }
}
struct Squares;
impl Store for Squares {
    type Ref<'a> = u64;
    fn get<'a>(&'a self, i: usize) -> u64 {
        (i * i) as u64
    }
}
fn dump<S: Store>(s: &S, n: usize) {
    for i in 0..n {
        print!("{:?} ", s.get(i));
    }
    println!();
}

fn main() {
    let mut arr = [1, 2, 3, 4, 5, 6];
    let mut w = WindowsMut { data: &mut arr, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[2] += win[0] + win[1];
        print!("{:?} ", win);
    }
    println!();
    println!("{:?}", arr);

    let mut lb = LineBuf { text: "first line\nsecond\n\nfourth".to_string(), off: 0 };
    while let Some((off, line)) = lb.next() {
        println!("@{} {:?} len={}", off, line, line.len());
    }
    let (a, b) = wrap_both::<OptionFamily>(3, "x");
    println!("{} {:?} {:?}", OptionFamily::name(), a, b);
    let (a, b) = wrap_both::<VecFamily>(4, "yy");
    println!("{} {:?} {:?}", VecFamily::name(), a, b);
    dump(&Strings(vec!["a".into(), "b".into()]), 3);
    dump(&Squares, 5);
}
