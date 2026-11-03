// PartialEq between trait objects via as_any, dyn equality, sorting dyn by key
use std::any::Any;
use std::fmt;

trait Value: fmt::Debug {
    fn as_any(&self) -> &dyn Any;
    fn eq_dyn(&self, other: &dyn Value) -> bool;
    fn rank(&self) -> i32;
    fn boxed_clone(&self) -> Box<dyn Value>;
}

impl<T> Value for T
where
    T: PartialEq + fmt::Debug + Clone + 'static + Ranked,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn eq_dyn(&self, other: &dyn Value) -> bool {
        other.as_any().downcast_ref::<T>().map_or(false, |o| o == self)
    }
    fn rank(&self) -> i32 {
        Ranked::rank(self)
    }
    fn boxed_clone(&self) -> Box<dyn Value> {
        Box::new(self.clone())
    }
}

trait Ranked {
    fn rank(&self) -> i32;
}
impl Ranked for i32 {
    fn rank(&self) -> i32 {
        *self
    }
}
impl Ranked for String {
    fn rank(&self) -> i32 {
        self.len() as i32
    }
}
#[derive(Debug, Clone, PartialEq)]
struct Pt {
    x: i32,
    y: i32,
}
impl Ranked for Pt {
    fn rank(&self) -> i32 {
        self.x.abs() + self.y.abs()
    }
}

impl PartialEq for dyn Value {
    fn eq(&self, other: &Self) -> bool {
        self.eq_dyn(other)
    }
}

impl Clone for Box<dyn Value> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl fmt::Display for dyn Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{:?} r={}>", self, self.rank())
    }
}

fn main() {
    let a: Box<dyn Value> = Box::new(5i32);
    let b: Box<dyn Value> = Box::new(5i32);
    let c: Box<dyn Value> = Box::new(String::from("5"));
    let d: Box<dyn Value> = Box::new(Pt { x: 1, y: -2 });
    let e: Box<dyn Value> = Box::new(Pt { x: 1, y: -2 });
    println!("{} {} {} {}", *a == *b, *a == *c, *d == *e, *d == *a);
    println!("{}", a.as_ref() != c.as_ref());
    let mut list: Vec<Box<dyn Value>> = vec![
        Box::new(Pt { x: 3, y: 3 }),
        Box::new(String::from("hello")),
        Box::new(-1i32),
        Box::new(String::from("ab")),
        Box::new(Pt { x: 0, y: 1 }),
        Box::new(4i32),
    ];
    list.sort_by_key(|v| v.rank());
    for v in &list {
        println!("{}", v);
    }
    let copy = list.clone();
    let all_eq = list.iter().zip(copy.iter()).all(|(x, y)| **x == **y);
    println!("clone eq {}", all_eq);
    let needle: Box<dyn Value> = Box::new(String::from("ab"));
    let pos = list.iter().position(|v| **v == *needle);
    println!("found at {:?}", pos);
    let needle2: &dyn Value = &Pt { x: 9, y: 9 };
    println!("contains {}", list.iter().any(|v| v.as_ref() == needle2));
    let mut dedup: Vec<Box<dyn Value>> = Vec::new();
    for v in vec![a, b, c, d, e] {
        if !dedup.iter().any(|x| **x == *v) {
            dedup.push(v);
        }
    }
    println!("dedup {:?}", dedup);
    let opt: Option<&dyn Value> = dedup.get(1).map(|b| b.as_ref());
    println!("{}", opt.map(|v| v.to_string()).unwrap_or_default());
}
