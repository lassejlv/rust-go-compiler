//~ ERROR E0005
fn main() {
    let v = vec![Some(1), None];
    for Some(x) in v { println!("{}", x); }
}
