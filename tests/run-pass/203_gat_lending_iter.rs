// Generic associated types: lending iterators and family patterns.

trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;

    fn for_each_lend<F>(mut self, mut f: F)
    where
        Self: Sized,
        F: for<'a> FnMut(Self::Item<'a>),
    {
        while let Some(x) = self.next() {
            f(x);
        }
    }
}

// Overlapping mutable windows over a slice.
struct WindowsMut<'s, T> {
    data: &'s mut [T],
    pos: usize,
    size: usize,
}

impl<'s, T> LendingIterator for WindowsMut<'s, T> {
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

// Line reader that reuses an internal buffer.
struct Lines {
    src: Vec<&'static str>,
    idx: usize,
    buf: String,
}

impl LendingIterator for Lines {
    type Item<'a> = &'a str where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a str> {
        let s = self.src.get(self.idx)?;
        self.idx += 1;
        self.buf.clear();
        for (i, w) in s.split_whitespace().enumerate() {
            if i > 0 {
                self.buf.push('_');
            }
            self.buf.push_str(&w.to_lowercase());
        }
        Some(&self.buf)
    }
}

// GAT "family" pattern: a container type constructor.
trait Family {
    type Member<T>;
    fn wrap<T>(t: T) -> Self::Member<T>;
}

struct OptionFamily;
struct VecFamily;

impl Family for OptionFamily {
    type Member<T> = Option<T>;
    fn wrap<T>(t: T) -> Option<T> {
        Some(t)
    }
}

impl Family for VecFamily {
    type Member<T> = Vec<T>;
    fn wrap<T>(t: T) -> Vec<T> {
        vec![t]
    }
}

fn wrap_both<F: Family>(a: i32, b: &'static str) -> (F::Member<i32>, F::Member<&'static str>) {
    (F::wrap(a), F::wrap(b))
}

// GAT with lifetime used for borrowed views
trait Store {
    type View<'a>: std::fmt::Debug
    where
        Self: 'a;
    fn view<'a>(&'a self, from: usize) -> Self::View<'a>;
}

struct Numbers(Vec<u32>);
impl Store for Numbers {
    type View<'a> = &'a [u32];
    fn view<'a>(&'a self, from: usize) -> &'a [u32] {
        &self.0[from.min(self.0.len())..]
    }
}

struct Text(String);
impl Store for Text {
    type View<'a> = Vec<&'a str>;
    fn view<'a>(&'a self, from: usize) -> Vec<&'a str> {
        self.0.split(',').skip(from).collect()
    }
}

fn show<S: Store>(s: &S) {
    for i in 0..3 {
        println!("  view({}) = {:?}", i, s.view(i));
    }
}

fn main() {
    let mut data = [1, 2, 3, 4, 5, 6];
    let mut w = WindowsMut { data: &mut data, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[2] += win[0] + win[1];
        println!("window {:?}", win);
    }
    println!("final {:?}", data);

    let lines = Lines {
        src: vec!["Hello World", "  GATs  are   Neat ", "x"],
        idx: 0,
        buf: String::new(),
    };
    let mut count = 0;
    lines.for_each_lend(|l| {
        count += 1;
        println!("line {}: {:?} ({} chars)", count, l, l.len());
    });

    let (a, b) = wrap_both::<OptionFamily>(5, "five");
    println!("{:?} {:?}", a, b);
    let (a, b) = wrap_both::<VecFamily>(6, "six");
    println!("{:?} {:?}", a, b);

    println!("numbers:");
    show(&Numbers(vec![10, 20, 30, 40]));
    println!("text:");
    show(&Text("a,b,c".to_string()));
}
