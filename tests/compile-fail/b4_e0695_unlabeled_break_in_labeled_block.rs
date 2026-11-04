//~ ERROR E0695
fn main() {
    let mut i = 0;
    loop {
        'check: {
            if i > 3 { break; }
            i += 1;
        }
    }
}
