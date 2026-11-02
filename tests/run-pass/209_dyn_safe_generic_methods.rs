// Object-safe traits that still offer generic convenience methods via `where Self: Sized`,
// in the style of serde's Serializer / erased_serde / tower's Service.
use std::fmt::Debug;

trait Sink {
    fn write_str(&mut self, s: &str);
    fn written(&self) -> usize;

    fn write_display<T: std::fmt::Display>(&mut self, t: T)
    where
        Self: Sized,
    {
        let s = t.to_string();
        self.write_str(&s);
    }

    fn write_all<I>(&mut self, items: I)
    where
        Self: Sized,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        for i in items {
            self.write_str(i.as_ref());
        }
    }

    fn boxed(self) -> Box<dyn Sink>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

struct StringSink {
    buf: String,
    count: usize,
}

impl Sink for StringSink {
    fn write_str(&mut self, s: &str) {
        if !self.buf.is_empty() {
            self.buf.push(' ');
        }
        self.buf.push_str(s);
        self.count += 1;
    }
    fn written(&self) -> usize {
        self.count
    }
}

struct CountSink(usize, usize);
impl Sink for CountSink {
    fn write_str(&mut self, s: &str) {
        self.0 += s.len();
        self.1 += 1;
    }
    fn written(&self) -> usize {
        self.1
    }
}

fn emit(s: &mut dyn Sink, words: &[&str]) {
    for w in words {
        s.write_str(w);
    }
}

// Trait with a generic method, made usable on dyn via an erased helper.
trait Visitor {
    fn visit_i64(&mut self, v: i64);
    fn visit_str(&mut self, v: &str);
}

trait Visitable {
    fn accept(&self, v: &mut dyn Visitor);
    fn accept_generic<V: Visitor>(&self, v: &mut V)
    where
        Self: Sized,
    {
        self.accept(v);
    }
}

impl Visitable for i64 {
    fn accept(&self, v: &mut dyn Visitor) {
        v.visit_i64(*self);
    }
}

impl Visitable for String {
    fn accept(&self, v: &mut dyn Visitor) {
        v.visit_str(self);
    }
}

impl<T: Visitable> Visitable for Vec<T> {
    fn accept(&self, v: &mut dyn Visitor) {
        v.visit_str("[");
        for x in self {
            x.accept(v);
        }
        v.visit_str("]");
    }
}

#[derive(Debug, Default)]
struct Printer {
    out: Vec<String>,
}

impl Visitor for Printer {
    fn visit_i64(&mut self, v: i64) {
        self.out.push(format!("i{}", v));
    }
    fn visit_str(&mut self, v: &str) {
        self.out.push(format!("s{:?}", v));
    }
}

#[derive(Debug, Default)]
struct Summer(i64, usize);
impl Visitor for Summer {
    fn visit_i64(&mut self, v: i64) {
        self.0 += v;
    }
    fn visit_str(&mut self, v: &str) {
        self.1 += v.len();
    }
}

fn visit_dyn(items: &[Box<dyn Visitable>]) -> (Printer, Summer) {
    let mut p = Printer::default();
    let mut s = Summer::default();
    for it in items {
        it.accept(&mut p);
        it.accept(&mut s);
    }
    (p, s)
}

fn show<T: Debug>(label: &str, t: T) {
    println!("{}: {:?}", label, t);
}

fn main() {
    let mut s = StringSink { buf: String::new(), count: 0 };
    s.write_display(42);
    s.write_display('x');
    s.write_all(vec!["a", "b"]);
    s.write_all(&[String::from("own")]);
    emit(&mut s, &["dyn1", "dyn2"]);
    println!("{:?} ({} writes)", s.buf, s.written());

    let mut sinks: Vec<Box<dyn Sink>> = vec![
        StringSink { buf: String::new(), count: 0 }.boxed(),
        CountSink(0, 0).boxed(),
    ];
    for sk in sinks.iter_mut() {
        emit(sk.as_mut(), &["hello", "there", "world"]);
        sk.write_str("!");
    }
    let counts: Vec<usize> = sinks.iter().map(|s| s.written()).collect();
    println!("{:?}", counts);

    let items: Vec<Box<dyn Visitable>> = vec![
        Box::new(5i64),
        Box::new(String::from("abc")),
        Box::new(vec![1i64, 2, 3]),
        Box::new(vec![String::from("x"), String::from("yz")]),
    ];
    let (p, s) = visit_dyn(&items);
    show("printer", p.out);
    show("summer", s);

    let mut p2 = Printer::default();
    vec![10i64, -3].accept_generic(&mut p2);
    String::from("direct").accept_generic(&mut p2);
    show("generic", p2.out);
}
