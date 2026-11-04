//~ ERROR E0382
fn consume_all<T>(item: T, sink: &mut Vec<T>) {
    for _ in 0..3 {
        sink.push(item);
    }
}
fn main() {}
