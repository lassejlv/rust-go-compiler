// Matching on &mut enum and mutating payloads in place.
#[derive(Debug)]
enum Shape {
    Circle { r: f64 },
    Rect { w: f64, h: f64 },
    Group(Vec<Shape>),
}

fn scale(s: &mut Shape, k: f64) {
    match s {
        Shape::Circle { r } => *r *= k,
        Shape::Rect { w, h } => {
            *w *= k;
            *h *= k;
        }
        Shape::Group(items) => {
            for it in items.iter_mut() {
                scale(it, k);
            }
            items.push(Shape::Circle { r: 0.5 });
        }
    }
}

fn main() {
    let mut g = Shape::Group(vec![Shape::Circle { r: 1.0 }, Shape::Rect { w: 2.0, h: 3.0 }]);
    scale(&mut g, 2.0);
    if let Shape::Group(ref mut v) = g {
        v.remove(0);
    }
    println!("{:?}", g);
}
