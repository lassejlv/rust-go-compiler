use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Click { x: i32, y: i32 },
    Key(char),
    Resize(u32, u32),
    Quit,
}

impl Event {
    fn kind(&self) -> &'static str {
        match self {
            Event::Click { .. } => "click",
            Event::Key(_) => "key",
            Event::Resize(..) => "resize",
            Event::Quit => "quit",
        }
    }
}

type Listener = Box<dyn Fn(&Event) -> bool>;

#[derive(Default)]
struct Bus {
    listeners: BTreeMap<&'static str, Vec<(u32, String, Listener)>>,
    any: Vec<Box<dyn FnMut(&Event)>>,
    next_id: u32,
}

impl Bus {
    fn on(&mut self, kind: &'static str, name: &str, f: impl Fn(&Event) -> bool + 'static) -> u32 {
        self.next_id += 1;
        self.listeners.entry(kind).or_default().push((self.next_id, name.to_string(), Box::new(f)));
        self.next_id
    }
    fn on_any(&mut self, f: impl FnMut(&Event) + 'static) {
        self.any.push(Box::new(f));
    }
    fn off(&mut self, id: u32) -> bool {
        for list in self.listeners.values_mut() {
            if let Some(pos) = list.iter().position(|(i, _, _)| *i == id) {
                list.remove(pos);
                return true;
            }
        }
        false
    }
    fn emit(&mut self, e: &Event) -> usize {
        for f in self.any.iter_mut() {
            f(e);
        }
        let mut handled = 0;
        if let Some(list) = self.listeners.get(e.kind()) {
            for (_, name, f) in list {
                handled += 1;
                if f(e) {
                    println!("    [{}] stopped propagation", name);
                    break;
                }
            }
        }
        handled
    }
}

fn main() {
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let mut bus = Bus::default();

    let l = log.clone();
    bus.on("click", "logger", move |e| {
        if let Event::Click { x, y } = e {
            l.borrow_mut().push(format!("click at ({}, {})", x, y));
        }
        false
    });
    let l = log.clone();
    let blocker = bus.on("click", "modal", move |e| {
        let stop = matches!(e, Event::Click { x, .. } if *x < 0);
        if stop {
            l.borrow_mut().push("modal ate click".to_string());
        }
        stop
    });
    let typed = Rc::new(RefCell::new(String::new()));
    let t = typed.clone();
    bus.on("key", "typist", move |e| {
        if let Event::Key(c) = e {
            if *c == '\u{8}' {
                t.borrow_mut().pop();
            } else {
                t.borrow_mut().push(*c);
            }
        }
        false
    });
    let size = Rc::new(RefCell::new((0u32, 0u32)));
    let s = size.clone();
    bus.on("resize", "layout", move |e| {
        if let Event::Resize(w, h) = e {
            *s.borrow_mut() = (*w, *h);
        }
        true
    });
    bus.on("resize", "never-called", |_| {
        println!("    this should not print");
        false
    });
    let counts = Rc::new(RefCell::new(BTreeMap::<&'static str, u32>::new()));
    let c = counts.clone();
    let mut seq = 0;
    bus.on_any(move |e| {
        seq += 1;
        *c.borrow_mut().entry(e.kind()).or_insert(0) += 1;
        if seq % 4 == 0 {
            println!("    (any-listener saw {} events)", seq);
        }
    });

    let events = vec![
        Event::Click { x: 10, y: 20 },
        Event::Key('h'),
        Event::Key('i'),
        Event::Click { x: -1, y: 5 },
        Event::Key('x'),
        Event::Key('\u{8}'),
        Event::Resize(800, 600),
        Event::Key('!'),
        Event::Quit,
    ];
    for (i, e) in events.iter().enumerate() {
        if i == 5 {
            println!("  removing modal: {}", bus.off(blocker));
            println!("  removing again: {}", bus.off(blocker));
        }
        let n = bus.emit(e);
        println!("emit {:?} -> {} listener(s)", e, n);
    }
    bus.emit(&Event::Click { x: -5, y: -5 });
    println!("log: {:?}", log.borrow());
    println!("typed: {:?}", typed.borrow());
    println!("size: {:?}", size.borrow());
    println!("counts: {:?}", counts.borrow());
    let total: usize = bus.listeners.values().map(|v| v.len()).sum();
    let names: Vec<&str> = bus.listeners.values().flatten().map(|(_, n, _)| n.as_str()).collect();
    println!("{} listeners: {:?}", total, names);
}
