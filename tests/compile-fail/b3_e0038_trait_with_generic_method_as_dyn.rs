//~ ERROR E0038
trait Serializer { fn write<T: std::fmt::Debug>(&self, t: T); }
fn use_it(_s: &dyn Serializer) {}
fn main() {}
