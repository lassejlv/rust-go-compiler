// A tiny entity-component-system with type-erased component storage
// (Box<dyn Any>, TypeId keys, downcasting), systems as trait objects and
// boxed closures, an event bus, resources, and a plugin trait with
// supertraits and default methods. Runs a deterministic particle/combat sim.
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Debug};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct Entity(u32);

trait Storage: Any {
    fn remove(&mut self, e: Entity) -> bool;
    fn len(&self) -> usize;
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct VecStorage<T> {
    dense: Vec<(Entity, T)>,
    index: HashMap<Entity, usize>,
}

impl<T: 'static> Storage for VecStorage<T> {
    fn remove(&mut self, e: Entity) -> bool {
        match self.index.remove(&e) {
            None => false,
            Some(i) => {
                self.dense.swap_remove(i);
                if i < self.dense.len() {
                    let moved = self.dense[i].0;
                    self.index.insert(moved, i);
                }
                true
            }
        }
    }
    fn len(&self) -> usize { self.dense.len() }
    fn type_name(&self) -> &'static str { std::any::type_name::<T>().rsplit("::").next().unwrap() }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[derive(Default)]
struct World {
    next: u32,
    alive: Vec<Entity>,
    storages: HashMap<TypeId, Box<dyn Storage>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq)]
enum Event {
    Spawned(Entity, &'static str),
    Hit { attacker: Entity, target: Entity, dmg: i32 },
    Died(Entity),
    Bounced(Entity),
}

impl World {
    fn spawn(&mut self) -> Entity {
        self.next += 1;
        let e = Entity(self.next);
        self.alive.push(e);
        e
    }
    fn storage<T: 'static>(&self) -> Option<&VecStorage<T>> {
        self.storages.get(&TypeId::of::<T>())?.as_any().downcast_ref::<VecStorage<T>>()
    }
    fn storage_mut<T: 'static>(&mut self) -> &mut VecStorage<T> {
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(VecStorage::<T> { dense: Vec::new(), index: HashMap::new() }))
            .as_any_mut()
            .downcast_mut::<VecStorage<T>>()
            .expect("type mismatch")
    }
    fn insert<T: 'static>(&mut self, e: Entity, c: T) -> &mut Self {
        let s = self.storage_mut::<T>();
        match s.index.get(&e) {
            Some(&i) => s.dense[i].1 = c,
            None => {
                s.index.insert(e, s.dense.len());
                s.dense.push((e, c));
            }
        }
        self
    }
    fn get<T: 'static>(&self, e: Entity) -> Option<&T> {
        let s = self.storage::<T>()?;
        s.index.get(&e).map(|&i| &s.dense[i].1)
    }
    fn get_mut<T: 'static>(&mut self, e: Entity) -> Option<&mut T> {
        let s = self.storage_mut::<T>();
        let i = *s.index.get(&e)?;
        Some(&mut s.dense[i].1)
    }
    fn despawn(&mut self, e: Entity) -> usize {
        self.alive.retain(|&x| x != e);
        self.storages.values_mut().map(|s| s.remove(e) as usize).sum()
    }
    fn query<T: 'static>(&self) -> Vec<Entity> {
        let mut v: Vec<Entity> = self.storage::<T>().map(|s| s.dense.iter().map(|(e, _)| *e).collect()).unwrap_or_default();
        v.sort();
        v
    }
    fn resource<R: 'static>(&self) -> Option<&R> { self.resources.get(&TypeId::of::<R>())?.downcast_ref() }
    fn resource_mut<R: 'static + Default>(&mut self) -> &mut R {
        self.resources.entry(TypeId::of::<R>()).or_insert_with(|| Box::new(R::default())).downcast_mut().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pos(f64, f64);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Vel(f64, f64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Health(i32);
#[derive(Debug, Clone, PartialEq, Eq)]
struct Name(String);
#[derive(Debug, Clone, Copy)]
struct Attack { power: i32, range: f64, cooldown: u32, ready_in: u32 }
#[derive(Default, Debug)]
struct Tick(u64);
#[derive(Default, Debug)]
struct Bounds { w: f64, h: f64 }
#[derive(Default)]
struct Stats { hits: u32, deaths: u32, bounces: u32, damage: i64 }

trait System {
    fn name(&self) -> String;
    fn run(&mut self, w: &mut World);
}

trait Plugin: Debug {
    fn id(&self) -> &'static str;
    fn build(&self, systems: &mut Vec<Box<dyn System>>);
    fn describe(&self) -> String { format!("plugin<{}>", self.id()) }
}

struct Movement;
impl System for Movement {
    fn name(&self) -> String { "movement".into() }
    fn run(&mut self, w: &mut World) {
        let (bw, bh) = w.resource::<Bounds>().map(|b| (b.w, b.h)).unwrap_or((100.0, 100.0));
        let ents = w.query::<Vel>();
        let mut bounced = vec![];
        for e in ents {
            let v = *w.get::<Vel>(e).unwrap();
            let Some(p) = w.get_mut::<Pos>(e) else { continue };
            p.0 += v.0;
            p.1 += v.1;
            let mut nv = v;
            if p.0 < 0.0 || p.0 > bw { nv.0 = -nv.0; p.0 = p.0.clamp(0.0, bw); }
            if p.1 < 0.0 || p.1 > bh { nv.1 = -nv.1; p.1 = p.1.clamp(0.0, bh); }
            if nv != v {
                bounced.push(e);
                w.insert(e, nv);
            }
        }
        for e in bounced { w.events.push(Event::Bounced(e)); }
    }
}

struct Combat;
impl System for Combat {
    fn name(&self) -> String { "combat".into() }
    fn run(&mut self, w: &mut World) {
        let attackers = w.query::<Attack>();
        let targets = w.query::<Health>();
        for a in attackers {
            if w.get::<Health>(a).map_or(false, |h| h.0 <= 0) { continue; }
            let atk = *w.get::<Attack>(a).unwrap();
            if atk.ready_in > 0 {
                w.get_mut::<Attack>(a).unwrap().ready_in -= 1;
                continue;
            }
            let pa = *w.get::<Pos>(a).unwrap();
            let best = targets.iter().copied().filter(|&t| t != a && w.get::<Health>(t).map_or(false, |h| h.0 > 0)).filter_map(|t| {
                let pt = w.get::<Pos>(t)?;
                let d = ((pa.0 - pt.0).powi(2) + (pa.1 - pt.1).powi(2)).sqrt();
                (d <= atk.range).then_some((d, t))
            }).min_by(|x, y| x.0.partial_cmp(&y.0).unwrap().then(x.1.cmp(&y.1)));
            if let Some((_, t)) = best {
                w.get_mut::<Health>(t).unwrap().0 -= atk.power;
                w.get_mut::<Attack>(a).unwrap().ready_in = atk.cooldown;
                w.events.push(Event::Hit { attacker: a, target: t, dmg: atk.power });
            }
        }
    }
}

struct Reaper;
impl System for Reaper {
    fn name(&self) -> String { "reaper".into() }
    fn run(&mut self, w: &mut World) {
        let dead: Vec<Entity> = w.query::<Health>().into_iter().filter(|&e| w.get::<Health>(e).unwrap().0 <= 0).collect();
        for e in dead {
            w.despawn(e);
            w.events.push(Event::Died(e));
        }
    }
}

struct FnSystem<F: FnMut(&mut World)> { label: &'static str, f: F }
impl<F: FnMut(&mut World)> System for FnSystem<F> {
    fn name(&self) -> String { format!("fn:{}", self.label) }
    fn run(&mut self, w: &mut World) { (self.f)(w) }
}

#[derive(Debug)]
struct CorePlugin;
#[derive(Debug)]
struct CombatPlugin { with_reaper: bool }

impl Plugin for CorePlugin {
    fn id(&self) -> &'static str { "core" }
    fn build(&self, s: &mut Vec<Box<dyn System>>) {
        s.push(Box::new(FnSystem { label: "tick", f: |w: &mut World| w.resource_mut::<Tick>().0 += 1 }));
        s.push(Box::new(Movement));
    }
}
impl Plugin for CombatPlugin {
    fn id(&self) -> &'static str { "combat" }
    fn build(&self, s: &mut Vec<Box<dyn System>>) {
        s.push(Box::new(Combat));
        if self.with_reaper { s.push(Box::new(Reaper)); }
    }
    fn describe(&self) -> String { format!("combat(reaper={})", self.with_reaper) }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &e in &self.alive {
            let name = self.get::<Name>(e).map(|n| n.0.as_str()).unwrap_or("?");
            write!(f, "  {:?} {:<7}", e, name)?;
            if let Some(p) = self.get::<Pos>(e) { write!(f, " pos=({:6.2},{:6.2})", p.0, p.1)?; }
            if let Some(h) = self.get::<Health>(e) { write!(f, " hp={:>3}", h.0)?; }
            if let Some(a) = self.get::<Attack>(e) { write!(f, " atk={}/{:.1}", a.power, a.range)?; }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn main() {
    let mut w = World::default();
    *w.resource_mut::<Bounds>() = Bounds { w: 20.0, h: 10.0 };
    let specs: [(&str, (f64, f64), (f64, f64), Option<i32>, Option<(i32, f64, u32)>); 6] = [
        ("knight", (2.0, 2.0), (1.0, 0.5), Some(30), Some((7, 3.0, 1))),
        ("archer", (18.0, 8.0), (-0.5, -0.25), Some(18), Some((4, 8.0, 0))),
        ("orc", (10.0, 5.0), (0.0, 0.0), Some(45), Some((9, 2.5, 2))),
        ("ghost", (5.0, 9.0), (1.5, 1.5), None, None),
        ("slime", (12.0, 1.0), (-0.75, 0.0), Some(12), None),
        ("rock", (0.0, 0.0), (0.0, 0.0), None, None),
    ];
    for (name, p, v, hp, atk) in specs {
        let e = w.spawn();
        w.insert(e, Name(name.to_string())).insert(e, Pos(p.0, p.1));
        if v != (0.0, 0.0) { w.insert(e, Vel(v.0, v.1)); }
        if let Some(h) = hp { w.insert(e, Health(h)); }
        if let Some((power, range, cooldown)) = atk { w.insert(e, Attack { power, range, cooldown, ready_in: 0 }); }
        w.events.push(Event::Spawned(e, name));
    }
    let mut names: Vec<(&'static str, usize)> = w.storages.values().map(|s| (s.type_name(), s.len())).collect();
    names.sort();
    println!("storages: {:?}", names);
    print!("{}", w);

    let plugins: Vec<Box<dyn Plugin>> = vec![Box::new(CorePlugin), Box::new(CombatPlugin { with_reaper: true })];
    let mut systems: Vec<Box<dyn System>> = Vec::new();
    for p in &plugins {
        p.build(&mut systems);
        println!("loaded {} -> {:?}", p.describe(), p);
    }
    let mut log_counter = 0usize;
    systems.push(Box::new(FnSystem { label: "drain-events", f: move |w: &mut World| {
        let evs = std::mem::take(&mut w.events);
        let tick = w.resource::<Tick>().map_or(0, |t| t.0);
        for ev in evs {
            let s = w.resource_mut::<Stats>();
            match &ev {
                Event::Hit { dmg, .. } => { s.hits += 1; s.damage += *dmg as i64; }
                Event::Died(_) => s.deaths += 1,
                Event::Bounced(_) => s.bounces += 1,
                Event::Spawned(..) => {}
            }
            if !matches!(ev, Event::Bounced(_)) && log_counter < 30 {
                log_counter += 1;
                println!("  t{:<3} {:?}", tick, ev);
            }
        }
    }}));
    println!("systems: {:?}", systems.iter().map(|s| s.name()).collect::<Vec<_>>());

    for _ in 0..40 {
        for s in systems.iter_mut() {
            s.run(&mut w);
        }
    }
    println!("after {} ticks:", w.resource::<Tick>().unwrap().0);
    print!("{}", w);
    let st = w.resource::<Stats>().unwrap();
    println!("hits={} deaths={} bounces={} damage={}", st.hits, st.deaths, st.bounces, st.damage);
    let mut names: BTreeMap<&'static str, usize> = BTreeMap::new();
    for s in w.storages.values() { names.insert(s.type_name(), s.len()); }
    println!("storages: {:?}", names);
    println!("missing resource: {:?}", w.resource::<String>());
    let boxed: Box<dyn Any> = Box::new(Health(5));
    println!("downcast Health: {:?}, as Pos: {:?}", boxed.downcast_ref::<Health>(), boxed.downcast_ref::<Pos>());
    match boxed.downcast::<Pos>() {
        Ok(p) => println!("unexpected {:?}", p),
        Err(b) => println!("still a Health: {}", b.is::<Health>()),
    }
    println!("{}", TypeId::of::<Pos>() == TypeId::of::<Pos>() && TypeId::of::<Pos>() != TypeId::of::<Vel>());
}
