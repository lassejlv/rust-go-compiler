//~ ERROR E0382
struct Person {
    name: String,
    age: u32,
}

fn show(p: Person) {
    println!("{} {}", p.name, p.age);
}

fn main() {
    let p = Person { name: String::from("Ann"), age: 30 };
    let n = p.name;
    println!("{}", n);
    show(p);
}
