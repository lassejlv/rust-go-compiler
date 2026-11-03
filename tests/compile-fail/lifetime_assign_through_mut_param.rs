//~ ERROR lifetime may not live long enough
fn set<'a, 'b>(slot: &mut &'a str, s: &'b str) {
    *slot = s;
}

fn main() {
    let mut x = "x";
    set(&mut x, "y");
    println!("{}", x);
}
