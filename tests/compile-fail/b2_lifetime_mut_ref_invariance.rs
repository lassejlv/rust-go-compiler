//~ ERROR lifetime
fn assign<'a, 'b>(dst: &mut &'a str, src: &'b str) {
    *dst = src;
}
fn main() { let mut s = "a"; assign(&mut s, "b"); println!("{}", s); }
