// `use` paths may start with a std prelude name.
use Option::{self as State, None as Ended, Some as InProgress};
fn f(x: State<i32>) -> i32 {
    match x {
        InProgress(v) => v,
        Ended => -1,
    }
}
fn main() {
    println!("{} {}", f(Some(3)), f(None));
}
