// Entity component system: type-erased component storages keyed by TypeId,
// generic queries, systems as boxed closures, and despawning.
use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Entity(u32);

trait Storage: Any {
    fn remove(&mut self, e: Entity) -> bool;
    fn len(&self) -> usize;
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct VecStorage<T> {
    items: BTreeMap<Entity, T>,
}

impl<T: 'static + Debug> Storage for VecStorage<T> {
    fn remove(&mut self, e: Entity) -> bool { self.items.remove(&e).is_some() }
    fn len(&self) -> usize { self.items.len() }
    fn type_name(&self) -> &'static str { std::any::type_name::<T>().rsplit("::").next().unwrap() }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[derive(Default)]
struct World {
    next: u32,
    alive: Vec<Entity>,
    stores: HashMap<TypeId, Box<dyn Storage>>,
    order: Vec<TypeId>,
}

impl World {
    fn spawn(&mut self) -> Entity {
        let e = Entity(self.next);
        self.next += 1;
        self.alive.push(e);
        e
    }
    fn insert<T: 'static + Debug>(&mut self, e: Entity, c: T) -> &mut Self {
        let id = TypeId::of::<T>();
        if !self.stores.contains_key(&id) {
            self.order.push(id);
        }
        let st = self
            .stores
            .entry(id)
            .or_insert_with(|| Box::new(VecStorage::<T> { items: BTreeMap::new() }));
        st.as_any_mut().downcast_mut::<VecStorage<T>>().unwrap().items.insert(e, c);
        self
    }
    fn storage<T: 'static>(&self) -> Option<&BTreeMap<Entity, T>> {
        self.stores.get(&TypeId::of::<T>())?.as_any().downcast_ref::<VecStorage<T>>().map(|s| &s.items)
    }
    fn storage_mut<T: 'static>(&mut self) -> Option<&mut BTreeMap<Entity, T>> {
        self.stores.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut::<VecStorage<T>>().map(|s| &mut s.items)
    }
    fn get<T: 'static>(&self, e: Entity) -> Option<&T> {
        self.storage::<T>()?.get(&e)
    }
    fn despawn(&mut self, e: Entity) -> usize {
        self.alive.retain(|&x| x != e);
        let mut n = 0;
        for id in &self.order {
            if self.stores.get_mut(id).unwrap().remove(e) {
                n += 1;
            }
        }
        n
    }
    fn query2<A: 'static + Clone, B: 'static + Clone>(&self) -> Vec<(Entity, A, B)> {
        let (Some(a), Some(b)) = (self.storage::<A>(), self.storage::<B>()) else {
            return vec![];
        };
        a.iter().filter_map(|(e, x)| b.get(e).map(|y| (*e, x.clone(), y.clone()))).collect()
    }
    fn summary(&self) -> String {
        self.order
            .iter()
            .map(|id| {
                let s = &self.stores[id];
                format!("{}:{}", s.type_name(), s.len())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pos { x: i32, y: i32 }
#[derive(Debug, Clone, Copy)]
struct Vel { dx: i32, dy: i32 }
#[derive(Debug, Clone)]
struct Name(String);
#[derive(Debug, Clone, Copy)]
struct Health(i32);
#[derive(Debug, Clone, Copy)]
struct Poisoned { per_tick: i32 }

type System = Box<dyn Fn(&mut World, u32) -> Vec<String>>;

fn main() {
    let mut w = World::default();
    let names = ["orc", "elf", "rock", "bat", "slime"];
    let mut ents = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let e = w.spawn();
        ents.push(e);
        w.insert(e, Name(n.to_string())).insert(e, Pos { x: i as i32 * 3, y: 0 });
        if *n != "rock" {
            w.insert(e, Vel { dx: (i as i32 % 3) - 1, dy: i as i32 % 2 + 1 });
            w.insert(e, Health(10 + i as i32 * 2));
        }
    }
    w.insert(ents[1], Poisoned { per_tick: 4 });
    w.insert(ents[4], Poisoned { per_tick: 7 });
    println!("summary: {}", w.summary());

    let systems: Vec<(&str, System)> = vec![
        ("movement", Box::new(|w: &mut World, _t| {
            let moves = w.query2::<Pos, Vel>();
            let pos = w.storage_mut::<Pos>().unwrap();
            for (e, p, v) in &moves {
                pos.insert(*e, Pos { x: p.x + v.dx, y: p.y + v.dy });
            }
            vec![format!("moved {}", moves.len())]
        })),
        ("poison", Box::new(|w: &mut World, t| {
            let hits = w.query2::<Poisoned, Health>();
            let mut log = Vec::new();
            for (e, p, h) in hits {
                let nh = h.0 - p.per_tick;
                w.storage_mut::<Health>().unwrap().insert(e, Health(nh));
                log.push(format!("t{} {} hp {}->{}", t, w.get::<Name>(e).unwrap().0, h.0, nh));
            }
            log
        })),
        ("reaper", Box::new(|w: &mut World, _| {
            let dead: Vec<Entity> = w
                .storage::<Health>()
                .map(|hs| hs.iter().filter(|(_, h)| h.0 <= 0).map(|(e, _)| *e).collect())
                .unwrap_or_default();
            dead.into_iter()
                .map(|e| {
                    let nm = w.get::<Name>(e).cloned().unwrap();
                    let removed = w.despawn(e);
                    format!("{} died ({} comps)", nm.0, removed)
                })
                .collect()
        })),
    ];

    for tick in 1..=4 {
        for (name, sys) in &systems {
            for line in sys(&mut w, tick) {
                println!("[{}:{}] {}", tick, name, line);
            }
        }
    }
    println!("summary: {}", w.summary());
    println!("alive: {:?}", w.alive);
    let mut rows: Vec<_> = w.query2::<Name, Pos>();
    rows.sort_by_key(|(_, _, p)| (p.y, p.x));
    for (e, n, p) in rows {
        let hp = w.get::<Health>(e).map(|h| h.0);
        println!("{:?} {:<6} at ({:>2},{:>2}) hp={:?}", e, n.0, p.x, p.y, hp);
    }
    println!("no Vel for rock: {:?}", w.get::<Vel>(ents[2]).is_none());
    println!("unknown type storage: {:?}", w.storage::<u64>().map(|s| s.len()));
    println!("{}", TypeId::of::<Pos>() == TypeId::of::<Pos>() && TypeId::of::<Pos>() != TypeId::of::<Vel>());
}
