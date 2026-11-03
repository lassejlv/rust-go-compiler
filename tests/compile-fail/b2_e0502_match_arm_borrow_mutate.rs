//~ ERROR E0502
fn main() {
    let mut v = vec![Some(1), None];
    match v.get(0) {
        Some(Some(x)) => { v.push(Some(*x)); println!("{}", x); }
        _ => {}
    }
}
