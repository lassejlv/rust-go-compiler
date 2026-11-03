//~ ERROR E0596
fn bump(x: &mut i32) {
    *x += 1;
}

fn main() {
    let n = 1;
    bump(&mut n);
}
