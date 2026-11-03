//~ ERROR E0621
fn first<'a>(x: &'a i32, y: &i32) -> &'a i32 {
    if *x > *y { x } else { y }
}

fn main() {
    println!("{}", first(&1, &2));
}
