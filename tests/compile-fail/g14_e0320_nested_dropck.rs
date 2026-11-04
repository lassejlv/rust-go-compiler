//~ ERROR E0320
#[derive(Debug)]
enum Nested<T> { Flat(T), Nest(Box<Nested<Vec<T>>>) }
fn main() {
    let n: Nested<u8> = Nested::Nest(Box::new(Nested::Flat(vec![1u8])));
    match n { Nested::Flat(_) => println!("flat"), Nested::Nest(_) => println!("nest") }
}
