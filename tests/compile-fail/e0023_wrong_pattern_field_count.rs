//~ ERROR E0023
enum Shape {
    Rect(u32, u32),
}

fn main() {
    let s = Shape::Rect(1, 2);
    match s {
        Shape::Rect(w) => println!("{}", w),
    }
}
