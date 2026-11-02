// A slotmap with generational keys, plus a secondary map, modeled after the slotmap crate.
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

trait Key: Copy + Eq + fmt::Debug {
    fn from_parts(idx: u32, version: u32) -> Self;
    fn idx(&self) -> u32;
    fn version(&self) -> u32;
}

macro_rules! new_key_type {
    ($($name:ident),*) => {
        $(
            #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
            struct $name { idx: u32, version: u32 }
            impl Key for $name {
                fn from_parts(idx: u32, version: u32) -> Self { $name { idx, version } }
                fn idx(&self) -> u32 { self.idx }
                fn version(&self) -> u32 { self.version }
            }
            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "{}({}v{})", stringify!($name), self.idx, self.version)
                }
            }
        )*
    };
}

new_key_type!(EntityId, WindowId);

enum Slot<T> {
    Occupied { version: u32, value: T },
    Vacant { version: u32, next_free: Option<u32> },
}

struct SlotMap<K: Key, V> {
    slots: Vec<Slot<V>>,
    free_head: Option<u32>,
    len: usize,
    _k: PhantomData<K>,
}

impl<K: Key, V> SlotMap<K, V> {
    fn with_key() -> Self {
        SlotMap { slots: Vec::new(), free_head: None, len: 0, _k: PhantomData }
    }

    fn insert(&mut self, value: V) -> K {
        self.insert_with_key(|_| value)
    }

    fn insert_with_key(&mut self, f: impl FnOnce(K) -> V) -> K {
        self.len += 1;
        if let Some(idx) = self.free_head {
            let slot = &mut self.slots[idx as usize];
            let (version, next) = match slot {
                Slot::Vacant { version, next_free } => (*version + 1, *next_free),
                Slot::Occupied { .. } => unreachable!(),
            };
            self.free_head = next;
            let key = K::from_parts(idx, version);
            *slot = Slot::Occupied { version, value: f(key) };
            key
        } else {
            let idx = self.slots.len() as u32;
            let key = K::from_parts(idx, 1);
            self.slots.push(Slot::Occupied { version: 1, value: f(key) });
            key
        }
    }

    fn remove(&mut self, key: K) -> Option<V> {
        let i = key.idx() as usize;
        match self.slots.get(i) {
            Some(Slot::Occupied { version, .. }) if *version == key.version() => {}
            _ => return None,
        }
        let old = std::mem::replace(
            &mut self.slots[i],
            Slot::Vacant { version: key.version(), next_free: self.free_head },
        );
        self.free_head = Some(key.idx());
        self.len -= 1;
        match old {
            Slot::Occupied { value, .. } => Some(value),
            _ => unreachable!(),
        }
    }

    fn get(&self, key: K) -> Option<&V> {
        match self.slots.get(key.idx() as usize)? {
            Slot::Occupied { version, value } if *version == key.version() => Some(value),
            _ => None,
        }
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        match self.slots.get_mut(key.idx() as usize)? {
            Slot::Occupied { version, value } if *version == key.version() => Some(value),
            _ => None,
        }
    }

    fn contains_key(&self, key: K) -> bool {
        self.get(key).is_some()
    }

    fn iter(&self) -> impl Iterator<Item = (K, &V)> + '_ {
        self.slots.iter().enumerate().filter_map(|(i, s)| match s {
            Slot::Occupied { version, value } => Some((K::from_parts(i as u32, *version), value)),
            Slot::Vacant { .. } => None,
        })
    }

    fn retain(&mut self, mut f: impl FnMut(K, &mut V) -> bool) {
        let keys: Vec<K> = self.iter().map(|(k, _)| k).collect();
        for k in keys {
            let keep = f(k, self.get_mut(k).unwrap());
            if !keep {
                self.remove(k);
            }
        }
    }
}

impl<K: Key, V> Index<K> for SlotMap<K, V> {
    type Output = V;
    fn index(&self, k: K) -> &V {
        self.get(k).expect("invalid key")
    }
}
impl<K: Key, V> IndexMut<K> for SlotMap<K, V> {
    fn index_mut(&mut self, k: K) -> &mut V {
        self.get_mut(k).expect("invalid key")
    }
}

struct SecondaryMap<K: Key, V> {
    slots: Vec<Option<(u32, V)>>,
    _k: PhantomData<K>,
}

impl<K: Key, V> SecondaryMap<K, V> {
    fn new() -> Self {
        SecondaryMap { slots: Vec::new(), _k: PhantomData }
    }
    fn insert(&mut self, k: K, v: V) {
        let i = k.idx() as usize;
        if self.slots.len() <= i {
            self.slots.resize_with(i + 1, || None);
        }
        self.slots[i] = Some((k.version(), v));
    }
    fn get(&self, k: K) -> Option<&V> {
        match self.slots.get(k.idx() as usize)? {
            Some((ver, v)) if *ver == k.version() => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Entity {
    name: String,
    hp: i32,
    me: Option<EntityId>,
}

fn main() {
    let mut sm: SlotMap<EntityId, Entity> = SlotMap::with_key();
    let a = sm.insert(Entity { name: "a".into(), hp: 10, me: None });
    let b = sm.insert_with_key(|k| Entity { name: "b".into(), hp: 5, me: Some(k) });
    let c = sm.insert(Entity { name: "c".into(), hp: 0, me: None });
    println!("{:?} {:?} {:?} len={}", a, b, c, sm.len);
    println!("b.me = {:?}", sm[b].me);
    sm[a].hp += 7;
    println!("a.hp = {}", sm[a].hp);

    let removed = sm.remove(b).map(|e| e.name);
    println!("removed {:?}, again {:?}", removed, sm.remove(b).map(|e| e.name));
    let d = sm.insert(Entity { name: "d".into(), hp: 3, me: None });
    println!("d = {:?} (reuses slot), b valid? {} d valid? {}", d, sm.contains_key(b), sm.contains_key(d));
    println!("stale get: {:?}", sm.get(b).map(|e| &e.name));

    let mut labels: SecondaryMap<EntityId, &str> = SecondaryMap::new();
    labels.insert(a, "label-a");
    labels.insert(d, "label-d");
    println!("{:?} {:?} {:?}", labels.get(a), labels.get(d), labels.get(b));

    sm.retain(|_, e| e.hp > 0);
    let names: Vec<String> = sm.iter().map(|(k, e)| format!("{:?}={}:{}", k, e.name, e.hp)).collect();
    println!("{:?} len={}", names, sm.len);

    let e = sm.insert(Entity { name: "e".into(), hp: 1, me: None });
    let f = sm.insert(Entity { name: "f".into(), hp: 1, me: None });
    println!("{:?} {:?}", e, f);

    let mut wins: SlotMap<WindowId, String> = SlotMap::with_key();
    let w1 = wins.insert("main".into());
    for _ in 0..3 {
        let k = wins.insert("tmp".into());
        wins.remove(k);
    }
    let w2 = wins.insert("popup".into());
    println!("{:?} {:?} {:?}", w1, w2, wins.iter().map(|(_, v)| v.as_str()).collect::<Vec<_>>());
}
