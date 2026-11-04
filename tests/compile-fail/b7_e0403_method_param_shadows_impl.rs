//~ ERROR E0403
// A method's type parameter may not shadow the impl's.
struct Wrapper<T>(T);

impl<T> Wrapper<T> {
    fn map<T>(self, x: T) -> T {
        x
    }
}

fn main() {
    println!("{}", Wrapper(1).map(2));
}
