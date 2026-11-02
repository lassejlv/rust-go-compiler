// Trait object upcasting (stable since 1.86): &dyn Sub -> &dyn Super, Box<dyn Sub> -> Box<dyn Super>,
// Rc<dyn Sub> -> Rc<dyn Super>, and supertrait methods called through every view.
use std::fmt;
use std::rc::Rc;

trait Named {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("hello from {}", self.name())
    }
}

trait Animal: Named {
    fn legs(&self) -> u32;
    fn sound(&self) -> &'static str;
}

trait Pet: Animal + fmt::Debug {
    fn owner(&self) -> &str;
    fn intro(&self) -> String {
        format!("{} ({} legs) owned by {} says {}", self.name(), self.legs(), self.owner(), self.sound())
    }
}

#[derive(Debug)]
struct Dog {
    owner: String,
}
#[derive(Debug)]
struct Parrot {
    owner: String,
    words: u32,
}

impl Named for Dog {
    fn name(&self) -> String {
        "Dog".into()
    }
}
impl Animal for Dog {
    fn legs(&self) -> u32 {
        4
    }
    fn sound(&self) -> &'static str {
        "woof"
    }
}
impl Pet for Dog {
    fn owner(&self) -> &str {
        &self.owner
    }
}

impl Named for Parrot {
    fn name(&self) -> String {
        format!("Parrot[{}]", self.words)
    }
    fn greet(&self) -> String {
        "squawk hello".into()
    }
}
impl Animal for Parrot {
    fn legs(&self) -> u32 {
        2
    }
    fn sound(&self) -> &'static str {
        "hello"
    }
}
impl Pet for Parrot {
    fn owner(&self) -> &str {
        &self.owner
    }
}

fn as_named(a: &dyn Animal) -> &dyn Named {
    a
}

fn count_legs(animals: &[Box<dyn Animal>]) -> u32 {
    animals.iter().map(|a| a.legs()).sum()
}

fn greet_all(items: Vec<Rc<dyn Named>>) {
    for (i, n) in items.iter().enumerate() {
        println!("  {}: {}", i, n.greet());
    }
}

fn main() {
    let pets: Vec<Box<dyn Pet>> = vec![
        Box::new(Dog { owner: "ann".into() }),
        Box::new(Parrot { owner: "bob".into(), words: 12 }),
    ];
    for p in &pets {
        println!("{}", p.intro());
        println!("{:?}", p);
        let a: &dyn Animal = p.as_ref();
        let n: &dyn Named = as_named(a);
        println!("upcast: {} / {}", a.sound(), n.greet());
    }

    // Box upcasting in a collection
    let animals: Vec<Box<dyn Animal>> = pets.into_iter().map(|p| p as Box<dyn Animal>).collect();
    println!("legs total = {}", count_legs(&animals));

    // Rc upcasting
    let rc_pet: Rc<dyn Pet> = Rc::new(Dog { owner: "cy".into() });
    let rc_animal: Rc<dyn Animal> = rc_pet.clone();
    let rc_named: Rc<dyn Named> = rc_animal.clone();
    println!("rc count = {}", Rc::strong_count(&rc_pet));
    println!("{} | {} | {}", rc_pet.owner(), rc_animal.legs(), rc_named.name());

    let parrot: Rc<dyn Animal> = Rc::new(Parrot { owner: "di".into(), words: 3 });
    greet_all(vec![rc_named, parrot as Rc<dyn Named>]);
    println!("rc count after = {}", Rc::strong_count(&rc_pet));

    // Option of upcast
    let maybe: Option<&dyn Pet> = None;
    let up: Option<&dyn Named> = maybe.map(|p| p as &dyn Named);
    println!("none upcast is_none = {}", up.is_none());
}
