//~ ERROR E0131
fn main<T: Default>() {
    let _x = T::default();
}
