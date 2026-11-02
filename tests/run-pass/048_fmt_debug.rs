#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Wrapper(i32, String);

#[derive(Debug)]
struct Empty;

#[derive(Debug)]
enum Shape {
    Circle(f64),
    Rect { w: f64, h: f64 },
    Nothing,
}

#[derive(Debug)]
struct Scene {
    name: &'static str,
    shapes: Vec<Shape>,
    origin: Option<Point>,
    tags: (char, bool),
}

fn main() {
    println!("{:?}", 42);
    println!("{:?}", -3.5);
    println!("{:?}", "quoted \"string\"\twith\nescapes");
    println!("{:?}", 'c');
    println!("{:?}", true);
    println!("{:?}", vec![1, 2, 3]);
    println!("{:?}", vec!["a", "b"]);
    println!("{:?}", Vec::<i32>::new());
    println!("{:?}", [1.0, 2.5]);
    println!("{:?}", (1, "x", 'y', 2.0));
    println!("{:?}", Some(5));
    println!("{:?}", None::<i32>);
    println!("{:?}", Some("text"));
    let ok: Result<i32, String> = Ok(1);
    let err: Result<i32, String> = Err("boom".to_string());
    println!("{:?} {:?}", ok, err);
    println!("{:?}", Point { x: 1, y: -2 });
    println!("{:?}", Wrapper(7, "seven".to_string()));
    println!("{:?}", Empty);
    println!("{:?}", Shape::Circle(1.0));
    println!("{:?}", Shape::Rect { w: 2.0, h: 3.5 });
    println!("{:?}", Shape::Nothing);
    let scene = Scene {
        name: "demo",
        shapes: vec![Shape::Circle(0.5), Shape::Nothing],
        origin: Some(Point { x: 0, y: 0 }),
        tags: ('t', false),
    };
    println!("{:?}", scene);
    println!("{:?}", vec![Some(1), None]);
    println!("{:?}", vec![vec![1, 2], vec![]]);
    println!("{:?}", &[Point { x: 1, y: 1 }]);
    let s = String::from("owned");
    println!("{:?} {:?}", s, &s);
    println!("{:?}", ());
    println!("{:?}", Box::new(5));
    println!("{:?}", "unicode é ✓");
    println!("{:?}", '\u{0}');
    println!("{:?}", "\u{7f}\r");
    println!("{:?}", 1u8 as char);
    println!("{:?}", 0.1 + 0.2);
    println!("{:?}", 1e100);
    println!("{:?}", i64::MIN);
}
