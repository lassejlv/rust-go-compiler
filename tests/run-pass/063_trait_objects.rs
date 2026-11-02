trait Animal {
    fn name(&self) -> &str;
    fn sound(&self) -> String;
    fn legs(&self) -> u32 {
        4
    }
}

struct Dog;
struct Bird {
    species: String,
}
struct Snake {
    length: u32,
}

impl Animal for Dog {
    fn name(&self) -> &str {
        "dog"
    }
    fn sound(&self) -> String {
        "woof".to_string()
    }
}

impl Animal for Bird {
    fn name(&self) -> &str {
        &self.species
    }
    fn sound(&self) -> String {
        "tweet".to_string()
    }
    fn legs(&self) -> u32 {
        2
    }
}

impl Animal for Snake {
    fn name(&self) -> &str {
        "snake"
    }
    fn sound(&self) -> String {
        "s".repeat(self.length as usize)
    }
    fn legs(&self) -> u32 {
        0
    }
}

fn introduce(a: &dyn Animal) {
    println!("The {} says {} and has {} legs", a.name(), a.sound(), a.legs());
}

fn total_legs(zoo: &[&dyn Animal]) -> u32 {
    zoo.iter().map(|a| a.legs()).sum()
}

fn loudest<'a>(zoo: &[&'a dyn Animal]) -> &'a dyn Animal {
    let mut best = zoo[0];
    for a in zoo.iter() {
        if a.sound().len() > best.sound().len() {
            best = *a;
        }
    }
    best
}

fn main() {
    let d = Dog;
    let b = Bird { species: "parrot".to_string() };
    let s = Snake { length: 5 };
    introduce(&d);
    introduce(&b);
    introduce(&s);
    let zoo: Vec<&dyn Animal> = vec![&d, &b, &s, &d];
    println!("total legs = {}", total_legs(&zoo));
    println!("loudest = {}", loudest(&zoo).name());
    let picked: &dyn Animal = if total_legs(&zoo) > 5 { &b } else { &s };
    println!("picked = {}", picked.name());
}
