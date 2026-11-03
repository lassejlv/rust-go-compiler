use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::collections::HashMap;

trait Event: Any + std::fmt::Debug {
    fn name(&self) -> &'static str;
}

#[derive(Debug)] struct KeyPress(char);
#[derive(Debug)] struct Click { x: i32, y: i32 }
#[derive(Debug)] struct Quit;
impl Event for KeyPress { fn name(&self) -> &'static str { "key" } }
impl Event for Click { fn name(&self) -> &'static str { "click" } }
impl Event for Quit { fn name(&self) -> &'static str { "quit" } }

struct Bus {
    handlers: HashMap<TypeId, Vec<Box<dyn Fn(&dyn Any) -> String>>>,
    order: Vec<TypeId>,
}

impl Bus {
    fn new() -> Self { Bus { handlers: HashMap::new(), order: vec![] } }
    fn subscribe<E: Event, F: Fn(&E) -> String + 'static>(&mut self, f: F) {
        let id = TypeId::of::<E>();
        if !self.handlers.contains_key(&id) { self.order.push(id); }
        self.handlers.entry(id).or_default().push(Box::new(move |a: &dyn Any| {
            f(a.downcast_ref::<E>().expect("type mismatch"))
        }));
    }
    fn publish<E: Event>(&self, e: &E) -> Vec<String> {
        match self.handlers.get(&TypeId::of::<E>()) {
            Some(hs) => hs.iter().map(|h| h(e as &dyn Any)).collect(),
            None => vec![format!("no handler for {}", e.name())],
        }
    }
}

// tiny ECS: component storage by type
struct World {
    next: u32,
    stores: HashMap<TypeId, Box<dyn Any>>,
}
impl World {
    fn new() -> Self { World { next: 0, stores: HashMap::new() } }
    fn spawn(&mut self) -> u32 { self.next += 1; self.next }
    fn insert<C: 'static>(&mut self, e: u32, c: C) {
        let store = self.stores.entry(TypeId::of::<C>()).or_insert_with(|| Box::new(BTreeMap::<u32, C>::new()));
        store.downcast_mut::<BTreeMap<u32, C>>().unwrap().insert(e, c);
    }
    fn get<C: 'static>(&self, e: u32) -> Option<&C> {
        self.stores.get(&TypeId::of::<C>())?.downcast_ref::<BTreeMap<u32, C>>()?.get(&e)
    }
    fn query<C: 'static>(&self) -> Vec<(u32, &C)> {
        match self.stores.get(&TypeId::of::<C>()) {
            Some(s) => s.downcast_ref::<BTreeMap<u32, C>>().unwrap().iter().map(|(k, v)| (*k, v)).collect(),
            None => vec![],
        }
    }
    fn update<C: 'static>(&mut self, f: impl Fn(u32, &mut C)) {
        if let Some(s) = self.stores.get_mut(&TypeId::of::<C>()) {
            for (k, v) in s.downcast_mut::<BTreeMap<u32, C>>().unwrap().iter_mut() { f(*k, v); }
        }
    }
}
#[derive(Debug, Clone, Copy)] struct Pos(i32, i32);
#[derive(Debug, Clone, Copy)] struct Vel(i32, i32);
#[derive(Debug)] struct Name(&'static str);

fn describe(v: &dyn Any) -> String {
    if let Some(i) = v.downcast_ref::<i32>() { format!("i32 {}", i) }
    else if let Some(s) = v.downcast_ref::<String>() { format!("String {:?}", s) }
    else if let Some(s) = v.downcast_ref::<&str>() { format!("&str {:?}", s) }
    else if v.is::<Pos>() { "a Pos".to_string() }
    else { "unknown".to_string() }
}

fn main() {
    let mut bus = Bus::new();
    bus.subscribe(|k: &KeyPress| format!("typed {}", k.0));
    bus.subscribe(|c: &Click| format!("clicked at {},{}", c.x, c.y));
    bus.subscribe(|k: &KeyPress| format!("upper {}", k.0.to_ascii_uppercase()));
    for s in bus.publish(&KeyPress('q')) { println!("{}", s); }
    for s in bus.publish(&Click { x: 3, y: -4 }) { println!("{}", s); }
    for s in bus.publish(&Quit) { println!("{}", s); }
    println!("{}", bus.order.len());

    let mut w = World::new();
    let a = w.spawn();
    let b = w.spawn();
    let c = w.spawn();
    w.insert(a, Pos(0, 0)); w.insert(a, Vel(1, 2)); w.insert(a, Name("alpha"));
    w.insert(b, Pos(10, 10)); w.insert(b, Name("beta"));
    w.insert(c, Pos(-5, 5)); w.insert(c, Vel(-1, 0));
    for _ in 0..3 {
        let vels: Vec<(u32, Vel)> = w.query::<Vel>().into_iter().map(|(e, v)| (e, *v)).collect();
        w.update::<Pos>(|e, p| {
            if let Some((_, v)) = vels.iter().find(|(id, _)| *id == e) { p.0 += v.0; p.1 += v.1; }
        });
    }
    for (e, p) in w.query::<Pos>() {
        let n = w.get::<Name>(e).map(|n| n.0).unwrap_or("?");
        println!("{} {} {:?} vel={:?}", e, n, p, w.get::<Vel>(e));
    }
    println!("{}", w.query::<String>().len());
    let items: Vec<Box<dyn Any>> = vec![Box::new(5i32), Box::new(String::from("s")), Box::new("lit"), Box::new(Pos(1, 1)), Box::new(1.5f64)];
    for i in &items { println!("{}", describe(i.as_ref())); }
    println!("{}", TypeId::of::<Vec<i32>>() == TypeId::of::<Vec<i32>>());
    println!("{}", TypeId::of::<Vec<i32>>() == TypeId::of::<Vec<u32>>());
}
