//~ ERROR E0382
fn consume<T: std::fmt::Debug>(t: T) { println!("{:?}", t); }
fn twice<T: std::fmt::Debug>(t: T) {
    consume(t);
    consume(t);
}
fn main() { twice(vec![1]); }
