//~ ERROR E0502
fn main() {
    let mut scores = vec![1, 2, 3];
    for s in scores.iter() {
        scores[0] += *s;
    }
}
