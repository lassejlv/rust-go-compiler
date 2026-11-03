//~ ERROR E0308
fn main() {
    let mut i = 0;
    let v = loop {
        i += 1;
        if i == 3 { break i; }
        if i == 5 { break "x"; }
    };
}
