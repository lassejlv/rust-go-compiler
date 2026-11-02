use mymacros::{trace, Describe};

trait Describe {
    fn describe() -> String;
}

#[derive(Describe, Debug)]
struct Point {
    #[describe(skip)]
    x: i32,
    y: i32,
    label: String,
}

mymacros::make_answer!(7);

#[trace(fast)]
fn compute(a: i32, b: i32) -> i32 {
    a * b + 1
}

fn main() {
    println!("{}", Point::describe());
    let p = Point { x: 1, y: 2, label: "p".to_string() };
    println!("{:?}", p);
    println!("answer = {}", answer());
    println!("compute = {}", compute(6, 7));
}
