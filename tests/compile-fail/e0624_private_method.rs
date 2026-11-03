//~ ERROR E0624
mod m {
    pub struct S;
    impl S {
        fn hidden(&self) -> i32 {
            1
        }
    }
}

fn main() {
    let s = m::S;
    println!("{}", s.hidden());
}
