//~ ERROR E0597
fn first_static<'a>(p: (&'static str, &'a str)) -> &'static str { p.0 }
fn main() {
    let r;
    {
        let o = String::from("own");
        r = first_static((&o, "x"));
    }
    println!("{}", r);
}
