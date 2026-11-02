struct Point {
    x: i32,
    y: i32,
}

struct Person {
    name: String,
    age: u32,
    email: String,
}

fn manhattan(p: &Point, q: &Point) -> i32 {
    (p.x - q.x).abs() + (p.y - q.y).abs()
}

fn make_person(name: &str, age: u32) -> Person {
    let email = format!("{}@example.com", name.to_lowercase());
    Person { name: name.to_string(), age, email }
}

fn main() {
    let p = Point { x: 3, y: -4 };
    println!("p = ({}, {})", p.x, p.y);
    let mut q = Point { x: 0, y: 0 };
    q.x = 10;
    q.y += 2;
    println!("q = ({}, {})", q.x, q.y);
    println!("manhattan = {}", manhattan(&p, &q));

    let alice = make_person("Alice", 30);
    println!("{} ({}) <{}>", alice.name, alice.age, alice.email);
    let bob = Person { name: String::from("Bob"), ..make_person("Template", 25) };
    println!("{} ({}) <{}>", bob.name, bob.age, bob.email);

    let Point { x, y } = p;
    println!("destructured: x={} y={}", x, y);
    let Point { x: px, .. } = q;
    println!("px = {}", px);

    let points = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }, Point { x: 5, y: 6 }];
    let mut sx = 0;
    let mut sy = 0;
    for pt in points.iter() {
        sx += pt.x;
        sy += pt.y;
    }
    println!("sum = ({}, {})", sx, sy);
}
