trait Describe {
    fn describe(&self) -> String;
    fn id(&self) -> u32;
}

struct Dog {
    name: String,
    age: u32,
}

struct Car {
    model: String,
    year: u32,
}

impl Describe for Dog {
    fn describe(&self) -> String {
        format!("{} the dog, {} years old", self.name, self.age)
    }
    fn id(&self) -> u32 {
        self.age * 100
    }
}

impl Describe for Car {
    fn describe(&self) -> String {
        format!("a {} from {}", self.model, self.year)
    }
    fn id(&self) -> u32 {
        self.year
    }
}

trait Area {
    fn area(&self) -> f64;
}

struct Square(f64);
struct Circle(f64);

impl Area for Square {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

impl Area for Circle {
    fn area(&self) -> f64 {
        3.0 * self.0 * self.0
    }
}

trait Double {
    fn double(&self) -> Self;
}

impl Double for i32 {
    fn double(&self) -> i32 {
        self * 2
    }
}

impl Double for String {
    fn double(&self) -> String {
        format!("{}{}", self, self)
    }
}

fn main() {
    let d = Dog { name: String::from("Rex"), age: 4 };
    let c = Car { model: String::from("Roadster"), year: 2008 };
    println!("{} (id {})", d.describe(), d.id());
    println!("{} (id {})", c.describe(), c.id());
    let s = Square(3.0);
    let ci = Circle(2.0);
    println!("square area = {}", s.area());
    println!("circle area = {}", ci.area());
    println!("sum = {}", s.area() + ci.area());
    println!("21.double() = {}", 21.double());
    println!("\"ab\".double() = {}", String::from("ab").double());
    println!("chained = {}", 5.double().double());
    println!("ufcs = {}", Describe::describe(&d));
    println!("qualified = {}", <Car as Describe>::id(&c));
}
