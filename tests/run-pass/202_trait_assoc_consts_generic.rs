use std::collections::BTreeMap;
use std::fmt::Debug;

trait Shape {
    const SIDES: u32;
    const NAME: &'static str;
    type Measure: Copy + Debug + PartialOrd + Into<f64>;

    fn area(&self) -> Self::Measure;

    fn describe(&self) -> String {
        format!("{}({} sides, area={:?})", Self::NAME, Self::SIDES, self.area())
    }

    fn scaled_area<F>(&self, f: F) -> f64
    where
        F: Fn(f64) -> f64,
    {
        f(self.area().into())
    }
}

struct Square(u32);
struct Tri {
    base: f32,
    height: f32,
}

impl Shape for Square {
    const SIDES: u32 = 4;
    const NAME: &'static str = "Square";
    type Measure = u32;
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Shape for Tri {
    const SIDES: u32 = 3;
    const NAME: &'static str = "Tri";
    type Measure = f32;
    fn area(&self) -> f32 {
        0.5 * self.base * self.height
    }
}

fn total_sides<A: Shape, B: Shape>() -> u32 {
    A::SIDES + B::SIDES
}

fn largest<S: Shape>(items: &[S]) -> Option<S::Measure> {
    let mut best: Option<S::Measure> = None;
    for it in items {
        let a = it.area();
        best = match best {
            None => Some(a),
            Some(b) if a > b => Some(a),
            keep => keep,
        };
    }
    best
}

// Trait with generic methods and an associated type used as a key.
trait Registry {
    type Key: Ord + Clone + Debug;
    fn insert_with<V: ToString, F: FnOnce(&Self::Key) -> V>(&mut self, key: Self::Key, f: F);
    fn get_all(&self) -> Vec<(Self::Key, String)>;
}

#[derive(Default)]
struct NumRegistry {
    map: BTreeMap<i32, String>,
}

impl Registry for NumRegistry {
    type Key = i32;
    fn insert_with<V: ToString, F: FnOnce(&i32) -> V>(&mut self, key: i32, f: F) {
        let v = f(&key).to_string();
        self.map.insert(key, v);
    }
    fn get_all(&self) -> Vec<(i32, String)> {
        self.map.iter().map(|(k, v)| (*k, v.clone())).collect()
    }
}

fn fill<R: Registry>(r: &mut R, keys: Vec<R::Key>) {
    for k in keys {
        r.insert_with(k, |k| format!("<{:?}>", k));
    }
}

// Associated consts used in const contexts and arrays
trait Packet {
    const SIZE: usize;
    const ID: u8;
    fn encode(&self) -> Vec<u8>;
}

struct Ping;
struct Data(u16, u16);

impl Packet for Ping {
    const SIZE: usize = 1;
    const ID: u8 = 0x01;
    fn encode(&self) -> Vec<u8> {
        vec![Self::ID]
    }
}

impl Packet for Data {
    const SIZE: usize = 5;
    const ID: u8 = 0x02;
    fn encode(&self) -> Vec<u8> {
        let mut v = vec![Self::ID];
        v.extend_from_slice(&self.0.to_be_bytes());
        v.extend_from_slice(&self.1.to_be_bytes());
        v
    }
}

fn check<P: Packet>(p: &P) -> bool {
    let e = p.encode();
    e.len() == P::SIZE && e[0] == P::ID
}

struct Buffer<const N: usize> {
    bytes: [u8; N],
}

const DATA_BUF: usize = <Data as Packet>::SIZE * 2;

fn main() {
    let sq = Square(7);
    let tr = Tri { base: 3.0, height: 5.0 };
    println!("{}", sq.describe());
    println!("{}", tr.describe());
    println!("{}", sq.scaled_area(|a| a * 1.5));
    println!("{}", tr.scaled_area(f64::sqrt));
    println!("sides total = {}", total_sides::<Square, Tri>());
    println!("{:?}", largest(&[Square(2), Square(9), Square(4)]));
    println!("{:?}", largest(&[Tri { base: 1.0, height: 1.0 }, Tri { base: 4.0, height: 2.0 }]));
    println!("{:?}", largest::<Square>(&[]));

    let mut r = NumRegistry::default();
    fill(&mut r, vec![5, -3, 12]);
    r.insert_with(0, |k| k * 100);
    println!("{:?}", r.get_all());

    println!("{} {}", check(&Ping), check(&Data(258, 65535)));
    println!("{:?}", Data(258, 65535).encode());
    let b = Buffer::<DATA_BUF> { bytes: [0xAB; DATA_BUF] };
    println!("buffer len {} first {:#x}", b.bytes.len(), b.bytes[0]);
    println!("{}", <Ping as Packet>::SIZE + <Data as Packet>::SIZE);
}
