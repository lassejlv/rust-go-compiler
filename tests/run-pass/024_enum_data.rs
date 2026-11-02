enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Triangle(f64, f64, f64),
    Point,
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle { radius } => 3.14159 * radius * radius,
            Shape::Rectangle { width, height } => width * height,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0;
                (s * (s - a) * (s - b) * (s - c)).sqrt()
            }
            Shape::Point => 0.0,
        }
    }

    fn name(&self) -> &str {
        match self {
            Shape::Circle { .. } => "circle",
            Shape::Rectangle { .. } => "rectangle",
            Shape::Triangle(..) => "triangle",
            Shape::Point => "point",
        }
    }
}

#[derive(Debug)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(u8, u8, u8),
}

fn process(msg: &Message) -> String {
    match msg {
        Message::Quit => String::from("quit"),
        Message::Move { x, y } => format!("move to {},{}", x, y),
        Message::Write(text) => format!("write '{}'", text),
        Message::ChangeColor(r, g, b) => format!("color {} {} {}", r, g, b),
    }
}

fn main() {
    let shapes = [
        Shape::Circle { radius: 1.5 },
        Shape::Rectangle { width: 3.0, height: 4.5 },
        Shape::Triangle(3.0, 4.0, 5.0),
        Shape::Point,
    ];
    let mut total = 0.0;
    for s in shapes.iter() {
        let a = s.area();
        println!("{}: {:.3}", s.name(), a);
        total += a;
    }
    println!("total area: {:.3}", total);

    let msgs = vec![
        Message::Move { x: 3, y: -1 },
        Message::Write(String::from("hello")),
        Message::ChangeColor(10, 20, 30),
        Message::Quit,
    ];
    for m in &msgs {
        println!("{} <- {:?}", process(m), m);
    }
}
