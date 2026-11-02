// Default methods calling each other, overridden at different levels,
// called via generics, dyn, and supertrait defaults that call subtrait-required methods.
trait Logger {
    fn prefix(&self) -> String {
        "LOG".to_string()
    }
    fn format_msg(&self, msg: &str) -> String {
        format!("[{}] {}", self.prefix(), self.transform(msg))
    }
    fn transform(&self, msg: &str) -> String {
        msg.to_string()
    }
    fn log(&self, msg: &str) -> String {
        let s = self.format_msg(msg);
        self.sink(&s);
        s
    }
    fn sink(&self, _s: &str) {}
    fn log_many(&self, msgs: &[&str]) -> Vec<String> {
        msgs.iter().map(|m| self.log(m)).collect()
    }
}

struct Plain;
impl Logger for Plain {}

struct Shouting;
impl Logger for Shouting {
    fn transform(&self, msg: &str) -> String {
        msg.to_uppercase() + "!"
    }
}

struct Tagged {
    tag: String,
    counter: std::cell::Cell<u32>,
}
impl Logger for Tagged {
    fn prefix(&self) -> String {
        format!("{}#{}", self.tag, self.counter.get())
    }
    fn sink(&self, _s: &str) {
        self.counter.set(self.counter.get() + 1);
    }
}

struct Reverse;
impl Logger for Reverse {
    fn format_msg(&self, msg: &str) -> String {
        msg.chars().rev().collect()
    }
}

trait Base {
    fn value(&self) -> i32;
    fn doubled(&self) -> i32 {
        self.value() * 2
    }
    fn describe(&self) -> String {
        format!("value={} doubled={}", self.value(), self.doubled())
    }
}
trait Derived: Base {
    fn extra(&self) -> i32 {
        self.doubled() + 1
    }
    fn full(&self) -> String {
        format!("{} extra={}", self.describe(), self.extra())
    }
}

struct N(i32);
impl Base for N {
    fn value(&self) -> i32 {
        self.0
    }
}
impl Derived for N {}

struct M(i32);
impl Base for M {
    fn value(&self) -> i32 {
        self.0
    }
    fn doubled(&self) -> i32 {
        self.0 * 10
    }
}
impl Derived for M {
    fn extra(&self) -> i32 {
        -1
    }
}

fn use_generic<L: Logger>(l: &L) -> String {
    l.log("generic")
}

fn main() {
    let loggers: Vec<Box<dyn Logger>> = vec![
        Box::new(Plain),
        Box::new(Shouting),
        Box::new(Tagged { tag: "T".into(), counter: std::cell::Cell::new(0) }),
        Box::new(Reverse),
    ];
    for l in &loggers {
        println!("{}", l.log("hello world"));
        for s in l.log_many(&["a", "bc"]) {
            println!("  {}", s);
        }
    }
    println!("{}", use_generic(&Shouting));
    let t = Tagged { tag: "G".into(), counter: std::cell::Cell::new(5) };
    println!("{}", use_generic(&t));
    println!("{}", use_generic(&t));
    println!("count {}", t.counter.get());

    let ds: Vec<&dyn Derived> = vec![&N(3), &M(4)];
    for d in ds {
        println!("{}", d.full());
    }
}
