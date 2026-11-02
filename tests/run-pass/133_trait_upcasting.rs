use std::fmt::Debug;

trait Named {
    fn name(&self) -> String;
}

trait Animal: Named {
    fn legs(&self) -> u32;
    fn summary(&self) -> String {
        format!("{} has {} legs", self.name(), self.legs())
    }
}

trait Pet: Animal + Debug {
    fn owner(&self) -> &str;
}

#[derive(Debug)]
struct Dog {
    owner: String,
}

#[derive(Debug)]
struct Spider;

impl Named for Dog {
    fn name(&self) -> String {
        "Dog".to_string()
    }
}
impl Animal for Dog {
    fn legs(&self) -> u32 {
        4
    }
}
impl Pet for Dog {
    fn owner(&self) -> &str {
        &self.owner
    }
}

impl Named for Spider {
    fn name(&self) -> String {
        "Spider".to_string()
    }
}
impl Animal for Spider {
    fn legs(&self) -> u32 {
        8
    }
    fn summary(&self) -> String {
        "a spider!".to_string()
    }
}
impl Pet for Spider {
    fn owner(&self) -> &str {
        "nobody"
    }
}

fn print_name(n: &dyn Named) {
    println!("name: {}", n.name());
}

fn print_animal(a: &dyn Animal) {
    println!("animal: {}", a.summary());
}

fn main() {
    let d = Dog { owner: "Sam".to_string() };
    let p: &dyn Pet = &d;
    println!("owner: {}", p.owner());
    println!("{:?}", p);
    let a: &dyn Animal = p;
    print_animal(a);
    let n: &dyn Named = a;
    print_name(n);
    print_name(p as &dyn Named);

    let pets: Vec<Box<dyn Pet>> = vec![Box::new(Dog { owner: "Kim".to_string() }), Box::new(Spider)];
    let mut animals: Vec<Box<dyn Animal>> = Vec::new();
    for pet in pets {
        println!("pet {:?} owned by {}", pet, pet.owner());
        animals.push(pet);
    }
    let total: u32 = animals.iter().map(|a| a.legs()).sum();
    println!("total legs {}", total);
    let names: Vec<Box<dyn Named>> = animals.into_iter().map(|a| a as Box<dyn Named>).collect();
    for n in &names {
        print_name(n.as_ref());
    }

    let rc: std::rc::Rc<dyn Pet> = std::rc::Rc::new(Spider);
    let rc_animal: std::rc::Rc<dyn Animal> = rc.clone();
    println!("{}", rc_animal.summary());
    println!("rc count {}", std::rc::Rc::strong_count(&rc));
}
