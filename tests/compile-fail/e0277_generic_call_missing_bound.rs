//~ ERROR E0277
// `describe` requires `T: Describe`, which `show`'s T is not bounded by
// (accepted per instance: i32 implements it).
trait Describe {
    fn describe(&self) -> String;
}
impl Describe for i32 {
    fn describe(&self) -> String { format!("int {}", self) }
}

fn describe<T: Describe>(t: &T) -> String {
    t.describe()
}

fn show<T>(t: T) -> String {
    describe(&t)
}

fn main() {
    println!("{}", show(3));
}
