use std::cell::RefCell;
use std::rc::Rc;

trait Observer {
    fn notify(&mut self, event: &str);
    fn report(&self) -> String;
}

struct Logger {
    lines: Vec<String>,
}

struct CountingObserver {
    count: usize,
    total_len: usize,
}

impl Observer for Logger {
    fn notify(&mut self, event: &str) {
        self.lines.push(event.to_uppercase());
    }
    fn report(&self) -> String {
        format!("Logger[{}]", self.lines.join(","))
    }
}

impl Observer for CountingObserver {
    fn notify(&mut self, event: &str) {
        self.count += 1;
        self.total_len += event.len();
    }
    fn report(&self) -> String {
        format!("Counter[count={}, len={}]", self.count, self.total_len)
    }
}

struct Subject {
    observers: Vec<Rc<RefCell<dyn Observer>>>,
}

impl Subject {
    fn emit(&self, e: &str) {
        for o in &self.observers {
            o.borrow_mut().notify(e);
        }
    }
}

trait Greeter {
    fn greet(&self) -> String;
}

struct English;
struct French {
    name: String,
}

impl Greeter for English {
    fn greet(&self) -> String {
        "hello".to_string()
    }
}
impl Greeter for French {
    fn greet(&self) -> String {
        format!("bonjour {}", self.name)
    }
}

fn greet_all(gs: &[Rc<dyn Greeter>]) -> Vec<String> {
    gs.iter().map(|g| g.greet()).collect()
}

fn main() {
    let english: Rc<dyn Greeter> = Rc::new(English);
    let french: Rc<dyn Greeter> = Rc::new(French { name: "Ana".to_string() });
    let list = vec![english.clone(), french.clone(), english.clone()];
    println!("{:?}", greet_all(&list));
    println!("english strong count = {}", Rc::strong_count(&english));
    drop(list);
    println!("english strong count = {}", Rc::strong_count(&english));

    let logger = Rc::new(RefCell::new(Logger { lines: vec![] }));
    let counter = Rc::new(RefCell::new(CountingObserver { count: 0, total_len: 0 }));
    let subject = Subject {
        observers: vec![logger.clone(), counter.clone(), logger.clone()],
    };
    subject.emit("start");
    subject.emit("tick");
    println!("{}", logger.borrow().report());
    println!("{}", counter.borrow().report());
    println!("logger rc = {}", Rc::strong_count(&logger));

    let as_dyn: Rc<RefCell<dyn Observer>> = counter.clone();
    as_dyn.borrow_mut().notify("direct");
    println!("{}", counter.borrow().report());
    for o in subject.observers.iter() {
        println!("-> {}", o.borrow().report());
    }

    // Rc<dyn Fn>
    let fns: Vec<Rc<dyn Fn(i32) -> i32>> = vec![Rc::new(|x| x + 1), Rc::new(|x| x * x), Rc::new(move |x| x - 100)];
    let shared = fns[1].clone();
    println!("{:?}", fns.iter().map(|f| f(7)).collect::<Vec<_>>());
    println!("shared(9) = {}", shared(9));
    println!("fn rc = {}", Rc::strong_count(&shared));
}
