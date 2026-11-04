// derived Debug interplay with manual Debug impls, debug_struct/tuple/map helpers and {:#?}
use std::fmt;

struct Secret(String);
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Secret(***{})", self.0.len()) }
}

struct Pair(i32, i32);
impl fmt::Debug for Pair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Pair").field(&self.0).field(&self.1).finish()
    }
}

struct Map(Vec<(&'static str, i32)>);
impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.0.iter().map(|(k, v)| (k, v))).finish()
    }
}

struct Partial { a: i32 }
impl fmt::Debug for Partial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Partial").field("a", &self.a).finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct User { name: &'static str, pw: Secret, pos: Pair, meta: Map, p: Partial, tags: Vec<&'static str>, nested: Option<Box<User>> }

#[derive(Debug)]
enum Event { Click { x: i32, y: i32 }, Key(char), Batch(Vec<Event>), Nothing }

fn main() {
    let u = User {
        name: "amy", pw: Secret("hunter2".into()), pos: Pair(1, -2),
        meta: Map(vec![("k", 1), ("j", 2)]), p: Partial { a: 5 }, tags: vec![],
        nested: Some(Box::new(User { name: "kid", pw: Secret(String::new()), pos: Pair(0, 0), meta: Map(vec![]), p: Partial { a: 0 }, tags: vec!["t"], nested: None })),
    };
    println!("{:?}", u);
    println!("{:#?}", u);
    let e = Event::Batch(vec![Event::Click { x: 1, y: 2 }, Event::Key('\n'), Event::Nothing, Event::Batch(vec![])]);
    println!("{:?}", e);
    println!("{:#?}", e);
    println!("{:>10?}|{:<6?}|", Pair(1, 2), Event::Nothing);
    println!("{:#?}", Map(vec![]));
    println!("{:#?}", (Partial { a: 1 }, [Pair(3, 4)]));
}
