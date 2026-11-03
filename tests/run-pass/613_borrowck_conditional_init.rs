// Deferred and conditional initialization.
fn main() {
    let c = std::env::args().count() > 100;
    let x;
    if c {
        x = 1;
    } else {
        x = 2;
    }
    let label: &str;
    match x {
        1 => label = "one",
        2 => label = "two",
        _ => label = "many",
    }
    let s: String;
    loop {
        s = String::from("from loop");
        break;
    }
    let y: i32;
    let r = if x > 0 {
        y = 5;
        y * 2
    } else {
        y = 0;
        y
    };
    println!("{} {} {} {} {}", x, label, s, y, r);
}
