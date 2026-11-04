// edition:2015
// Edition 2015 trait-object syntax in many positions: inherent impls on bare
// trait objects (`impl Trait { .. }`), `Box<Trait + Send + 'static>`,
// `&mut Iterator<Item = T>`, trait objects in generics and associated
// types, and blanket impls over `Trait` objects.
use std::fmt;
use std::rc::Rc;
use std::collections::BTreeMap;

trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> &'static str;
    fn legs(&self) -> u32 { 4 }
}

// inherent methods on the trait object type `dyn Animal` written the 2015 way
impl Animal {
    fn introduce(&self) -> String {
        format!("{} says {} on {} legs", self.name(), self.sound(), self.legs())
    }
}

impl fmt::Debug for Animal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Animal({})", self.name())
    }
}

struct Dog { name: String }
struct Bird;
struct Snake { length: f32 }

impl Animal for Dog {
    fn name(&self) -> String { self.name.clone() }
    fn sound(&self) -> &'static str { "woof" }
}
impl Animal for Bird {
    fn name(&self) -> String { "bird".to_string() }
    fn sound(&self) -> &'static str { "tweet" }
    fn legs(&self) -> u32 { 2 }
}
impl Animal for Snake {
    fn name(&self) -> String { format!("snake({}m)", self.length) }
    fn sound(&self) -> &'static str { "hiss" }
    fn legs(&self) -> u32 { 0 }
}

fn total_legs(zoo: &[Rc<Animal>]) -> u32 { zoo.iter().map(|a| a.legs()).sum() }

fn drain_sum(it: &mut Iterator<Item = u32>) -> u32 {
    let mut s = 0;
    while let Some(x) = it.next() {
        s += x;
    }
    s
}

fn spawn_job(f: Box<Fn() -> String + Send + 'static>) -> String { f() }

trait Registry {
    type Item: ?Sized;
    fn get(&self, key: &str) -> Option<&Self::Item>;
}

struct Zoo { animals: BTreeMap<String, Box<Animal>> }

impl Registry for Zoo {
    type Item = Animal;
    fn get(&self, key: &str) -> Option<&(Animal + 'static)> { self.animals.get(key).map(|b| &**b) }
}

trait Loud { fn loud(&self) -> String; }
impl<'a> Loud for Animal + 'a {
    fn loud(&self) -> String { self.sound().to_uppercase() }
}

fn pick<'a>(a: &'a Animal, b: &'a Animal) -> &'a Animal {
    if a.legs() >= b.legs() { a } else { b }
}

fn main() {
    let zoo: Vec<Rc<Animal>> = vec![
        Rc::new(Dog { name: "rex".to_string() }),
        Rc::new(Bird),
        Rc::new(Snake { length: 2.5 }),
    ];
    for a in &zoo {
        println!("{}", a.introduce());
    }
    println!("legs: {}", total_legs(&zoo));
    println!("{:?}", zoo);
    let shared = zoo[0].clone();
    println!("rc count: {}", Rc::strong_count(&shared));

    let mut it = vec![1u32, 2, 3, 4].into_iter();
    println!("drained: {}", drain_sum(&mut it));
    let mut evens = (0..10u32).filter(|x| x % 2 == 0);
    println!("evens: {}", drain_sum(&mut evens));

    let label = String::from("job");
    println!("{}", spawn_job(Box::new(move || format!("{} done", label))));

    let mut animals: BTreeMap<String, Box<Animal>> = BTreeMap::new();
    animals.insert("d".to_string(), Box::new(Dog { name: "fido".to_string() }));
    animals.insert("s".to_string(), Box::new(Snake { length: 0.75 }));
    let z = Zoo { animals: animals };
    for k in &["d", "s", "x"] {
        match z.get(k) {
            Some(a) => println!("{} -> {} / {}", k, a.introduce(), a.loud()),
            None => println!("{} -> none", k),
        }
    }
    let b = Bird;
    let s = Snake { length: 1.0 };
    println!("pick: {}", pick(&b, &s).name());
}
