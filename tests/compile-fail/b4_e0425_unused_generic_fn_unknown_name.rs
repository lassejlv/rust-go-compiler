//~ ERROR E0425
fn total<T: Into<u64>>(items: Vec<T>) -> u64 {
    let mut sum = 0;
    for x in items { sum += x.into(); }
    sum + offset
}
fn main() {}
