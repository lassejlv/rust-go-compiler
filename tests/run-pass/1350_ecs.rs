// ECS-style simulation: type-erased component storages (Box<dyn Any>),
// generic queries, systems as trait objects, events, a Rc/Weak scene graph,
// f32 physics, and Drop-order logging.
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Entity {
    id: u32,
    gen: u32,
}

trait Storage: Any {
    fn remove(&mut self, e: u32) -> bool;
    fn len(&self) -> usize;
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct VecStorage<T> {
    dense: Vec<(u32, T)>,
    sparse: HashMap<u32, usize>,
}

impl<T: 'static> VecStorage<T> {
    fn new() -> Self {
        VecStorage { dense: Vec::new(), sparse: HashMap::new() }
    }
    fn insert(&mut self, e: u32, v: T) -> Option<T> {
        if let Some(&i) = self.sparse.get(&e) {
            return Some(std::mem::replace(&mut self.dense[i].1, v));
        }
        self.sparse.insert(e, self.dense.len());
        self.dense.push((e, v));
        None
    }
    fn get(&self, e: u32) -> Option<&T> {
        self.sparse.get(&e).map(|&i| &self.dense[i].1)
    }
    fn get_mut(&mut self, e: u32) -> Option<&mut T> {
        match self.sparse.get(&e) {
            Some(&i) => Some(&mut self.dense[i].1),
            None => None,
        }
    }
}

impl<T: 'static> Storage for VecStorage<T> {
    fn remove(&mut self, e: u32) -> bool {
        match self.sparse.remove(&e) {
            None => false,
            Some(i) => {
                self.dense.swap_remove(i);
                if i < self.dense.len() {
                    let moved = self.dense[i].0;
                    self.sparse.insert(moved, i);
                }
                true
            }
        }
    }
    fn len(&self) -> usize {
        self.dense.len()
    }
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>().rsplit("::").next().unwrap()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
struct World {
    gens: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
    storages: HashMap<TypeId, Box<dyn Storage>>,
    events: RefCell<Vec<Event>>,
    tick: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Spawned(Entity),
    Despawned(Entity),
    Collision(Entity, Entity),
    Bounced { e: Entity, wall: &'static str },
}

impl World {
    fn spawn(&mut self) -> Entity {
        let e = if let Some(id) = self.free.pop() {
            self.gens[id as usize] += 1;
            self.alive[id as usize] = true;
            Entity { id, gen: self.gens[id as usize] }
        } else {
            self.gens.push(0);
            self.alive.push(true);
            Entity { id: self.gens.len() as u32 - 1, gen: 0 }
        };
        self.events.borrow_mut().push(Event::Spawned(e));
        e
    }
    fn is_alive(&self, e: Entity) -> bool {
        self.alive.get(e.id as usize).copied().unwrap_or(false) && self.gens[e.id as usize] == e.gen
    }
    fn despawn(&mut self, e: Entity) -> bool {
        if !self.is_alive(e) {
            return false;
        }
        self.alive[e.id as usize] = false;
        self.free.push(e.id);
        for s in self.storages.values_mut() {
            s.remove(e.id);
        }
        self.events.borrow_mut().push(Event::Despawned(e));
        true
    }
    fn storage<T: 'static>(&self) -> Option<&VecStorage<T>> {
        self.storages.get(&TypeId::of::<T>()).and_then(|s| s.as_any().downcast_ref::<VecStorage<T>>())
    }
    fn storage_mut<T: 'static>(&mut self) -> &mut VecStorage<T> {
        self.storages.entry(TypeId::of::<T>()).or_insert_with(|| Box::new(VecStorage::<T>::new())).as_any_mut().downcast_mut::<VecStorage<T>>().expect("type mismatch")
    }
    fn add<T: 'static>(&mut self, e: Entity, c: T) -> &mut Self {
        assert!(self.is_alive(e), "adding component to dead entity {:?}", e);
        self.storage_mut::<T>().insert(e.id, c);
        self
    }
    fn get<T: 'static>(&self, e: Entity) -> Option<&T> {
        if !self.is_alive(e) {
            return None;
        }
        self.storage::<T>()?.get(e.id)
    }
    fn entity(&self, id: u32) -> Entity {
        Entity { id, gen: self.gens[id as usize] }
    }
    /// Visit every entity that has both A and B, with mutable access to A.
    fn join2<A: 'static, B: 'static + Clone>(&mut self, mut f: impl FnMut(Entity, &mut A, &B)) {
        let bs: Vec<(u32, B)> = match self.storage::<B>() {
            Some(s) => s.dense.iter().map(|(e, b)| (*e, b.clone())).collect(),
            None => return,
        };
        let gens = self.gens.clone();
        let sa = self.storage_mut::<A>();
        for (id, b) in bs {
            if let Some(a) = sa.get_mut(id) {
                f(Entity { id, gen: gens[id as usize] }, a, &b);
            }
        }
    }
    fn emit(&self, ev: Event) {
        self.events.borrow_mut().push(ev);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pos(f32, f32);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vel(f32, f32);
#[derive(Debug, Clone, Copy)]
struct Radius(f32);
#[derive(Debug, Clone)]
struct Name(String);
#[derive(Debug, Clone, Copy)]
struct Health(i32);

trait System {
    fn name(&self) -> &str;
    fn run(&mut self, w: &mut World);
}

struct Movement {
    dt: f32,
}
impl System for Movement {
    fn name(&self) -> &str {
        "movement"
    }
    fn run(&mut self, w: &mut World) {
        let dt = self.dt;
        w.join2::<Pos, Vel>(|_, p, v| {
            p.0 += v.0 * dt;
            p.1 += v.1 * dt;
        });
    }
}

struct Gravity(f32);
impl System for Gravity {
    fn name(&self) -> &str {
        "gravity"
    }
    fn run(&mut self, w: &mut World) {
        let g = self.0;
        let ids: Vec<u32> = w.storage::<Vel>().map(|s| s.dense.iter().map(|(e, _)| *e).collect()).unwrap_or_default();
        for id in ids {
            if let Some(v) = w.storage_mut::<Vel>().get_mut(id) {
                v.1 -= g;
            }
        }
    }
}

struct Walls {
    w: f32,
    h: f32,
    bounces: usize,
}
impl System for Walls {
    fn name(&self) -> &str {
        "walls"
    }
    fn run(&mut self, world: &mut World) {
        let (wd, ht) = (self.w, self.h);
        let mut hits = Vec::new();
        let pos: Vec<(u32, Pos)> = world.storage::<Pos>().unwrap().dense.clone();
        for (id, p) in pos {
            let e = world.entity(id);
            let r = world.get::<Radius>(e).map_or(0.0, |r| r.0);
            if let Some(v) = world.storage_mut::<Vel>().get_mut(id) {
                let mut np = p;
                if p.0 - r < 0.0 || p.0 + r > wd {
                    v.0 = -v.0;
                    np.0 = p.0.clamp(r, wd - r);
                    hits.push((e, if p.0 - r < 0.0 { "left" } else { "right" }, np));
                }
                if p.1 - r < 0.0 || p.1 + r > ht {
                    v.1 = -v.1 * 0.9;
                    np.1 = p.1.clamp(r, ht - r);
                    hits.push((e, if p.1 - r < 0.0 { "floor" } else { "ceiling" }, np));
                }
            }
        }
        for (e, wall, np) in hits {
            self.bounces += 1;
            world.storage_mut::<Pos>().insert(e.id, np);
            world.emit(Event::Bounced { e, wall });
        }
    }
}

struct Collisions {
    pairs_checked: u64,
}
impl System for Collisions {
    fn name(&self) -> &str {
        "collisions"
    }
    fn run(&mut self, w: &mut World) {
        let mut items: Vec<(u32, Pos, f32)> = Vec::new();
        if let (Some(ps), Some(rs)) = (w.storage::<Pos>(), w.storage::<Radius>()) {
            for (id, p) in &ps.dense {
                if let Some(r) = rs.get(*id) {
                    items.push((*id, *p, r.0));
                }
            }
        }
        items.sort_by(|a, b| a.1 .0.partial_cmp(&b.1 .0).unwrap().then(a.0.cmp(&b.0)));
        let mut damaged = Vec::new();
        for i in 0..items.len() {
            for j in i + 1..items.len() {
                let (a, b) = (&items[i], &items[j]);
                if b.1 .0 - a.1 .0 > a.2 + b.2 {
                    break; // sweep and prune on x
                }
                self.pairs_checked += 1;
                let (dx, dy) = (a.1 .0 - b.1 .0, a.1 .1 - b.1 .1);
                if dx * dx + dy * dy <= (a.2 + b.2) * (a.2 + b.2) {
                    let (ea, eb) = (w.entity(a.0), w.entity(b.0));
                    w.emit(Event::Collision(ea.min(eb), ea.max(eb)));
                    damaged.push(ea);
                    damaged.push(eb);
                }
            }
        }
        for e in damaged {
            if let Some(h) = w.storage_mut::<Health>().get_mut(e.id) {
                h.0 -= 7;
            }
        }
    }
}

struct Reaper {
    reaped: Vec<String>,
}
impl System for Reaper {
    fn name(&self) -> &str {
        "reaper"
    }
    fn run(&mut self, w: &mut World) {
        let dead: Vec<u32> = w.storage::<Health>().map(|s| s.dense.iter().filter(|(_, h)| h.0 <= 0).map(|(e, _)| *e).collect()).unwrap_or_default();
        let mut dead = dead;
        dead.sort();
        for id in dead {
            let e = w.entity(id);
            let n = w.get::<Name>(e).map(|n| n.0.clone()).unwrap_or_else(|| format!("#{}", id));
            if w.despawn(e) {
                self.reaped.push(format!("{}@{}", n, w.tick));
            }
        }
    }
}

// ---- scene graph with Rc/Weak ----
struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
    local: Cell<(f32, f32)>,
    drops: Rc<RefCell<Vec<String>>>,
}

impl Node {
    fn new(name: &str, local: (f32, f32), drops: &Rc<RefCell<Vec<String>>>) -> Rc<Node> {
        Rc::new(Node { name: name.into(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]), local: Cell::new(local), drops: drops.clone() })
    }
    fn attach(parent: &Rc<Node>, child: Rc<Node>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(child);
    }
    fn world_pos(&self) -> (f32, f32) {
        let (x, y) = self.local.get();
        match self.parent.borrow().upgrade() {
            Some(p) => {
                let (px, py) = p.world_pos();
                (px + x, py + y)
            }
            None => (x, y),
        }
    }
    fn path(&self) -> String {
        match self.parent.borrow().upgrade() {
            Some(p) => format!("{}/{}", p.path(), self.name),
            None => self.name.clone(),
        }
    }
    fn walk(self: &Rc<Self>, depth: usize, out: &mut Vec<String>) {
        let (x, y) = self.world_pos();
        out.push(format!("{}{} @ ({:.1}, {:.1}) rc={} weak={}", "  ".repeat(depth), self.name, x, y, Rc::strong_count(self), Rc::weak_count(self)));
        for c in self.children.borrow().iter() {
            c.walk(depth + 1, out);
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.drops.borrow_mut().push(self.name.clone());
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "e{}v{}", self.id, self.gen)
    }
}

fn main() {
    let mut w = World::default();
    let mut seed = 12345u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f32 / 1000.0
    };
    for i in 0..12 {
        let e = w.spawn();
        w.add(e, Pos(10.0 + rnd() * 80.0, 10.0 + rnd() * 40.0)).add(e, Vel(rnd() * 20.0 - 10.0, rnd() * 10.0)).add(e, Radius(1.0 + rnd() * 3.0)).add(e, Health(20)).add(e, Name(format!("ball{}", i)));
    }
    let wall_e = w.spawn();
    w.add(wall_e, Pos(50.0, 0.0)).add(wall_e, Name("anchor".to_string()));
    let mut systems: Vec<Box<dyn System>> = vec![Box::new(Gravity(0.5)), Box::new(Movement { dt: 0.1 }), Box::new(Walls { w: 100.0, h: 60.0, bounces: 0 }), Box::new(Collisions { pairs_checked: 0 }), Box::new(Reaper { reaped: vec![] })];
    let mut event_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut first_events = Vec::new();
    for t in 0..400 {
        w.tick = t;
        for s in systems.iter_mut() {
            s.run(&mut w);
        }
        if t % 50 == 0 && t > 0 {
            let e = w.spawn();
            w.add(e, Pos(50.0, 30.0)).add(e, Vel(0.0, 0.0)).add(e, Radius(2.0)).add(e, Health(5));
        }
        for ev in w.events.borrow_mut().drain(..) {
            let k = match ev {
                Event::Spawned(_) => "spawned",
                Event::Despawned(_) => "despawned",
                Event::Collision(..) => "collision",
                Event::Bounced { .. } => "bounced",
            };
            *event_counts.entry(k).or_default() += 1;
            if first_events.len() < 6 && !matches!(ev, Event::Spawned(_)) {
                first_events.push(format!("t{} {:?}", t, ev));
            }
        }
    }
    println!("{:?}", event_counts);
    for e in &first_events {
        println!("{}", e);
    }
    println!("system order: {}", systems.iter().map(|s| s.name()).collect::<Vec<_>>().join(" -> "));
    let mut storage_info: Vec<String> = w.storages.values().map(|s| format!("{}:{}", s.type_name(), s.len())).collect();
    storage_info.sort();
    println!("storages: {:?}", storage_info);
    let alive: Vec<Entity> = (0..w.gens.len() as u32).map(|id| w.entity(id)).filter(|&e| w.is_alive(e)).collect();
    println!("alive: {}", alive.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(","));
    for e in alive.iter().take(5) {
        let p = w.get::<Pos>(*e).unwrap();
        println!("  {} {:?} pos=({:.3}, {:.3}) hp={:?} v={:?}", e, w.get::<Name>(*e).map(|n| n.0.as_str()), p.0, p.1, w.get::<Health>(*e).map(|h| h.0), w.get::<Vel>(*e).map(|v| (v.0 * 1000.0).round() / 1000.0));
    }
    let stale = Entity { id: 0, gen: 99 };
    println!("stale get: {:?}", w.get::<Pos>(stale));
    println!("despawn stale: {}", w.despawn(stale));
    let sum: f32 = w.storage::<Pos>().unwrap().dense.iter().map(|(_, p)| p.0 + p.1).sum();
    println!("pos checksum {:.2}", sum);
    let f: f32 = 0.1 + 0.2;
    println!("f32 fmt: {} {:?} {} {:e} {} {}", f, f, 1.0f32 / 3.0, 1234.5f32, f32::MAX, f32::MIN_POSITIVE);
    println!("f32 more: {} {} {} {:?}", 16777217.0f32, 1e-7f32, 3.4e38f32 * 10.0, (2.5f32).powf(0.5));

    // scene graph
    let drops = Rc::new(RefCell::new(Vec::new()));
    {
        let root = Node::new("root", (0.0, 0.0), &drops);
        let arm = Node::new("arm", (1.0, 2.0), &drops);
        let hand = Node::new("hand", (0.5, 0.25), &drops);
        let finger = Node::new("finger", (0.125, 0.0), &drops);
        Node::attach(&hand, finger.clone());
        Node::attach(&arm, hand.clone());
        Node::attach(&root, arm.clone());
        Node::attach(&root, Node::new("leg", (-1.0, -3.0), &drops));
        let mut out = vec![];
        root.walk(0, &mut out);
        println!("{}", out.join("\n"));
        println!("finger path {} world {:?}", finger.path(), finger.world_pos());
        arm.local.set((10.0, 10.0));
        println!("after move: {:?}", finger.world_pos());
        drop(hand);
        arm.children.borrow_mut().clear();
        println!("finger orphan? parent alive = {}", finger.parent.borrow().upgrade().is_some());
        println!("drops so far: {:?}", drops.borrow());
    }
    println!("drops at end: {:?}", drops.borrow());
}
