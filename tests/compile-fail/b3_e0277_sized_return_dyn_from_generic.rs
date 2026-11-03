//~ ERROR E0277
trait Animal {}
fn make<T: ?Sized + Animal>(t: Box<T>) -> T { *t }
fn main() {}
