// GPUI-style App with typed global state: HashMap<TypeId, Box<dyn Any>> with typed accessors,
// update_global with a lease pattern, observers keyed by TypeId, and default-initialized globals.
use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;

trait Global: 'static {}

#[derive(Debug, Default)]
struct Theme {
    name: String,
    font_size: f32,
}
impl Global for Theme {}

#[derive(Debug, Default)]
struct Settings {
    vim_mode: bool,
    tab_size: u32,
}
impl Global for Settings {}

#[derive(Debug, Default)]
struct Counter(u64);
impl Global for Counter {}

type Observer = Box<dyn FnMut(&App) -> String>;

struct App {
    globals: HashMap<TypeId, Box<dyn Any>>,
    observers: HashMap<TypeId, Vec<Observer>>,
    pending_notifications: Vec<TypeId>,
    log: Vec<String>,
}

impl App {
    fn new() -> Self {
        App { globals: HashMap::new(), observers: HashMap::new(), pending_notifications: vec![], log: vec![] }
    }

    fn set_global<G: Global>(&mut self, g: G) {
        self.globals.insert(TypeId::of::<G>(), Box::new(g));
        self.notify::<G>();
    }

    fn has_global<G: Global>(&self) -> bool {
        self.globals.contains_key(&TypeId::of::<G>())
    }

    fn global<G: Global>(&self) -> &G {
        self.globals
            .get(&TypeId::of::<G>())
            .and_then(|b| b.downcast_ref::<G>())
            .unwrap_or_else(|| panic!("no global {}", type_name::<G>()))
    }

    fn try_global<G: Global>(&self) -> Option<&G> {
        self.globals.get(&TypeId::of::<G>())?.downcast_ref()
    }

    fn global_mut<G: Global>(&mut self) -> &mut G {
        self.globals.get_mut(&TypeId::of::<G>()).and_then(|b| b.downcast_mut::<G>()).expect("missing global")
    }

    fn default_global<G: Global + Default>(&mut self) -> &mut G {
        self.globals.entry(TypeId::of::<G>()).or_insert_with(|| Box::new(G::default())).downcast_mut::<G>().unwrap()
    }

    // Lease the global out of the map so the closure can borrow both it and the app mutably.
    fn update_global<G: Global, R>(&mut self, f: impl FnOnce(&mut G, &mut App) -> R) -> R {
        let id = TypeId::of::<G>();
        let mut leased = self.globals.remove(&id).expect("global not set");
        let r = f(leased.downcast_mut::<G>().unwrap(), self);
        self.globals.insert(id, leased);
        self.notify::<G>();
        r
    }

    fn remove_global<G: Global>(&mut self) -> Option<G> {
        self.globals.remove(&TypeId::of::<G>()).map(|b| *b.downcast::<G>().unwrap())
    }

    fn observe_global<G: Global>(&mut self, f: impl FnMut(&App) -> String + 'static) {
        self.observers.entry(TypeId::of::<G>()).or_default().push(Box::new(f));
    }

    fn notify<G: Global>(&mut self) {
        self.pending_notifications.push(TypeId::of::<G>());
    }

    fn flush_effects(&mut self) {
        while !self.pending_notifications.is_empty() {
            let ids = std::mem::take(&mut self.pending_notifications);
            for id in ids {
                if let Some(mut obs) = self.observers.remove(&id) {
                    for o in obs.iter_mut() {
                        let msg = o(self);
                        self.log.push(msg);
                    }
                    self.observers.insert(id, obs);
                }
            }
        }
    }
}

fn short_name<T>() -> &'static str {
    let n = type_name::<T>();
    n.rsplit("::").next().unwrap()
}

fn main() {
    let mut app = App::new();
    app.observe_global::<Theme>(|app| {
        let t = app.global::<Theme>();
        format!("theme changed -> {} @ {}", t.name, t.font_size)
    });
    let mut seen = 0;
    app.observe_global::<Settings>(move |app| {
        seen += 1;
        format!("settings observer call #{}: {:?}", seen, app.global::<Settings>())
    });

    app.set_global(Theme { name: "One Dark".into(), font_size: 14.0 });
    app.set_global(Settings { vim_mode: false, tab_size: 4 });
    app.flush_effects();

    println!("has Theme={} Counter={}", app.has_global::<Theme>(), app.has_global::<Counter>());
    println!("try Counter = {:?}", app.try_global::<Counter>());
    app.default_global::<Counter>().0 += 5;
    app.default_global::<Counter>().0 += 5;
    println!("counter = {:?}", app.global::<Counter>());

    let size = app.update_global::<Theme, f32>(|theme, app| {
        theme.font_size += 2.0;
        theme.name.push_str(" (modified)");
        app.global_mut::<Counter>().0 *= 3;
        theme.font_size
    });
    println!("new size {} counter {}", size, app.global::<Counter>().0);
    app.global_mut::<Settings>().vim_mode = true;
    app.notify::<Settings>();
    app.update_global::<Settings, ()>(|s, _| s.tab_size = 2);
    app.flush_effects();

    for l in &app.log {
        println!("log: {}", l);
    }
    let removed = app.remove_global::<Counter>();
    println!("removed {:?}, has now {}", removed, app.has_global::<Counter>());
    println!("{} {} {}", short_name::<Theme>(), short_name::<Settings>(), short_name::<Vec<u8>>());
    println!("{}", TypeId::of::<Theme>() == TypeId::of::<Theme>());
    println!("{}", TypeId::of::<Theme>() == TypeId::of::<Settings>());
    let anys: Vec<Box<dyn Any>> = vec![Box::new(1u8), Box::new("str"), Box::new(Theme::default())];
    for a in &anys {
        let desc = if let Some(v) = a.downcast_ref::<u8>() {
            format!("u8 {}", v)
        } else if let Some(s) = a.downcast_ref::<&str>() {
            format!("&str {}", s)
        } else if a.is::<Theme>() {
            "a Theme".to_string()
        } else {
            "unknown".to_string()
        };
        println!("{}", desc);
    }
}
