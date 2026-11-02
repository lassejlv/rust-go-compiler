// Event dispatch with Box<dyn Fn(&Event, &mut App)> listeners, capture/bubble phases over an
// element hierarchy, propagation stopping, typed listeners via Any downcasting, and hit testing.
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct MouseDown {
    x: i32,
    y: i32,
    button: u8,
}
#[derive(Debug, Clone)]
struct KeyDown {
    key: String,
    ctrl: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    Capture,
    Bubble,
}

struct App {
    log: Vec<String>,
    propagate: bool,
    counters: HashMap<&'static str, i32>,
}

type AnyListener = Box<dyn Fn(&dyn Any, Phase, &mut App)>;

struct Node {
    name: &'static str,
    rect: (i32, i32, i32, i32), // x, y, w, h
    children: Vec<usize>,
    listeners: HashMap<TypeId, Vec<AnyListener>>,
}

struct Window {
    nodes: Vec<Node>,
}

impl Window {
    fn add(&mut self, parent: Option<usize>, name: &'static str, rect: (i32, i32, i32, i32)) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node { name, rect, children: vec![], listeners: HashMap::new() });
        if let Some(p) = parent {
            self.nodes[p].children.push(id);
        }
        id
    }

    fn on<E: 'static>(&mut self, node: usize, f: impl Fn(&E, Phase, &mut App) + 'static) {
        let wrapped: AnyListener = Box::new(move |ev, phase, app| {
            if let Some(e) = ev.downcast_ref::<E>() {
                f(e, phase, app)
            }
        });
        self.nodes[node].listeners.entry(TypeId::of::<E>()).or_default().push(wrapped);
    }

    fn hit_path(&self, x: i32, y: i32) -> Vec<usize> {
        let mut path = vec![0];
        let mut cur = 0;
        'outer: loop {
            // last child wins (top-most)
            for &c in self.nodes[cur].children.iter().rev() {
                let (rx, ry, rw, rh) = self.nodes[c].rect;
                if x >= rx && x < rx + rw && y >= ry && y < ry + rh {
                    path.push(c);
                    cur = c;
                    continue 'outer;
                }
            }
            break;
        }
        path
    }

    fn dispatch<E: 'static>(&self, path: &[usize], ev: &E, app: &mut App) {
        app.propagate = true;
        let tid = TypeId::of::<E>();
        for &n in path {
            if let Some(ls) = self.nodes[n].listeners.get(&tid) {
                for l in ls {
                    l(ev, Phase::Capture, app);
                    if !app.propagate {
                        return;
                    }
                }
            }
        }
        for &n in path.iter().rev() {
            if let Some(ls) = self.nodes[n].listeners.get(&tid) {
                for l in ls {
                    l(ev, Phase::Bubble, app);
                    if !app.propagate {
                        return;
                    }
                }
            }
        }
    }
}

fn main() {
    let mut w = Window { nodes: vec![] };
    let root = w.add(None, "root", (0, 0, 100, 100));
    let sidebar = w.add(Some(root), "sidebar", (0, 0, 30, 100));
    let editor = w.add(Some(root), "editor", (30, 0, 70, 100));
    let button = w.add(Some(sidebar), "button", (5, 5, 20, 10));
    let overlay = w.add(Some(editor), "overlay", (50, 50, 30, 30));

    for (id, name) in [(root, "root"), (sidebar, "sidebar"), (editor, "editor"), (button, "button"), (overlay, "overlay")] {
        w.on::<MouseDown>(id, move |e, phase, app| {
            if phase == Phase::Bubble {
                app.log.push(format!("{} bubble mouse({},{} b{})", name, e.x, e.y, e.button));
            }
        });
    }
    w.on::<MouseDown>(root, |_, phase, app| {
        if phase == Phase::Capture {
            app.log.push("root capture mouse".into());
        }
    });
    w.on::<MouseDown>(button, |e, phase, app| {
        if phase == Phase::Bubble && e.button == 0 {
            *app.counters.entry("clicks").or_insert(0) += 1;
            app.log.push("button clicked -> stop".into());
            app.propagate = false;
        }
    });
    w.on::<MouseDown>(overlay, |e, phase, app| {
        if phase == Phase::Capture && e.button == 2 {
            app.log.push("overlay swallowed right click in capture".into());
            app.propagate = false;
        }
    });
    w.on::<KeyDown>(editor, |k, phase, app| {
        if phase == Phase::Bubble {
            let entry = app.counters.entry("keys").or_insert(0);
            *entry += 1;
            app.log.push(format!("editor key {:?}{}", k.key, if k.ctrl { " +ctrl" } else { "" }));
            if k.ctrl && k.key == "s" {
                app.log.push("save!".into());
                app.propagate = false;
            }
        }
    });
    w.on::<KeyDown>(root, |k, phase, app| {
        app.log.push(format!("root {:?} key {}", phase, k.key));
    });

    let mut app = App { log: vec![], propagate: true, counters: HashMap::new() };
    let clicks = [(10, 8, 0), (10, 8, 1), (60, 60, 0), (60, 60, 2), (40, 10, 0), (99, 99, 0)];
    for (x, y, b) in clicks {
        let path = w.hit_path(x, y);
        let names: Vec<&str> = path.iter().map(|&i| w.nodes[i].name).collect();
        app.log.push(format!("-- click at ({},{}) path {:?}", x, y, names));
        w.dispatch(&path, &MouseDown { x, y, button: b }, &mut app);
    }
    let focus_path = vec![root, editor];
    for (k, ctrl) in [("a", false), ("s", true)] {
        app.log.push(format!("-- key {}", k));
        w.dispatch(&focus_path, &KeyDown { key: k.into(), ctrl }, &mut app);
    }
    // dispatching an event type with no listeners is a no-op
    w.dispatch(&focus_path, &42u32, &mut app);
    for l in &app.log {
        println!("{}", l);
    }
    let mut counts: Vec<_> = app.counters.iter().collect();
    counts.sort();
    println!("{:?}", counts);
}
