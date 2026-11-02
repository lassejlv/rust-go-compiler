trait Greeter {
    fn name(&self) -> String;

    fn greeting(&self) -> String {
        format!("Hello, {}!", self.name())
    }

    fn shout(&self) -> String {
        self.greeting().to_uppercase()
    }
}

struct English {
    who: String,
}

struct Pirate;

impl Greeter for English {
    fn name(&self) -> String {
        self.who.clone()
    }
}

impl Greeter for Pirate {
    fn name(&self) -> String {
        "matey".to_string()
    }
    fn greeting(&self) -> String {
        format!("Ahoy, {}!", self.name())
    }
}

trait Counter {
    fn count(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.count() == 0
    }
    fn describe(&self) -> String {
        match self.count() {
            0 => "none".to_string(),
            1 => "one".to_string(),
            n => format!("{} items", n),
        }
    }
}

struct Bag {
    items: Vec<i32>,
}

impl Counter for Bag {
    fn count(&self) -> usize {
        self.items.len()
    }
}

fn main() {
    let e = English { who: "Alice".to_string() };
    let p = Pirate;
    println!("{}", e.greeting());
    println!("{}", e.shout());
    println!("{}", p.greeting());
    println!("{}", p.shout());
    let bags = [Bag { items: vec![] }, Bag { items: vec![1] }, Bag { items: vec![1, 2, 3] }];
    for b in bags.iter() {
        println!("count={} empty={} describe={}", b.count(), b.is_empty(), b.describe());
    }
}
