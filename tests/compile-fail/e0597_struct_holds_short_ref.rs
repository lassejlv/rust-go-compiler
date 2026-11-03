//~ ERROR E0597
struct Holder<'a> {
    r: &'a i32,
}

fn main() {
    let h;
    {
        let v = 10;
        h = Holder { r: &v };
    }
    println!("{}", h.r);
}
