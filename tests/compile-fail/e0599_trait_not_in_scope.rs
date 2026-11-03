//~ ERROR E0599
mod m {
    pub trait Shout {
        fn shout(&self) -> String;
    }
    impl Shout for str {
        fn shout(&self) -> String {
            self.to_uppercase()
        }
    }
}

fn main() {
    println!("{}", "hi".shout());
}
