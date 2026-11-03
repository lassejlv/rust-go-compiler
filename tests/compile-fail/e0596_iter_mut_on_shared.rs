//~ ERROR E0596
fn double(v: &[i32]) {
    for x in v.iter_mut() {
        *x *= 2;
    }
}

fn main() {
    double(&[1, 2]);
}
