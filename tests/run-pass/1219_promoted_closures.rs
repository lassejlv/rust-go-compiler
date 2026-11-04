fn pick(n: u8) -> &'static dyn Fn(i32) -> i32 {
    match n {
        0 => &|x| x + 1,
        1 => &|x| x * 2,
        _ => &|x| -x,
    }
}
fn table() -> Vec<(&'static str, &'static dyn Fn(f64) -> f64)> {
    vec![("sq", &|x| x * x), ("half", &|x| x / 2.0)]
}
fn main() {
    let f: &'static dyn Fn() = &|| println!("hi");
    f();
    println!("{} {} {}", pick(0)(5), pick(1)(5), pick(9)(5));
    for (n, g) in table() { println!("{} {}", n, g(3.0)); }
}
