use std::fmt;

trait Named {
    fn name(&self) -> String;
}

trait Greeter: Named {
    fn greet(&self) -> String {
        format!("Hi, I'm {}", self.name())
    }
}

trait Printable: fmt::Display {
    fn print_boxed(&self) {
        let s = self.to_string();
        let line = "-".repeat(s.len() + 2);
        println!("+{}+", line);
        println!("| {} |", s);
        println!("+{}+", line);
    }
}

trait Entity: Named + fmt::Debug {
    fn id(&self) -> u32;
    fn tag(&self) -> String {
        format!("#{}:{}", self.id(), self.name())
    }
}

#[derive(Debug)]
struct Robot {
    serial: u32,
}

impl Named for Robot {
    fn name(&self) -> String {
        format!("RX-{}", self.serial)
    }
}

impl Greeter for Robot {}

impl Entity for Robot {
    fn id(&self) -> u32 {
        self.serial
    }
}

impl fmt::Display for Robot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Robot {}", self.serial)
    }
}

impl Printable for Robot {}

#[derive(Debug)]
struct Human {
    first: String,
}

impl Named for Human {
    fn name(&self) -> String {
        self.first.clone()
    }
}

impl Greeter for Human {
    fn greet(&self) -> String {
        format!("Hello there, {} here", self.name())
    }
}

impl Entity for Human {
    fn id(&self) -> u32 {
        self.first.len() as u32
    }
}

fn all_tags(es: &[&dyn Entity]) -> Vec<String> {
    es.iter().map(|e| e.tag()).collect()
}

fn introduce<T: Greeter + Entity>(x: &T) {
    println!("{} ({:?}) tag={}", x.greet(), x, x.tag());
}

fn main() {
    let r = Robot { serial: 42 };
    let h = Human { first: "Ada".to_string() };
    introduce(&r);
    introduce(&h);
    r.print_boxed();
    println!("{:?}", all_tags(&[&r, &h]));
    let greeters: Vec<Box<dyn Greeter>> = vec![Box::new(Robot { serial: 7 }), Box::new(Human { first: "Linus".to_string() })];
    for g in &greeters {
        println!("{} / name={}", g.greet(), g.name());
    }
}
