//~ ERROR E0597
fn main() {
    let mut longest: Option<&String> = None;
    for i in 0..3 {
        let s = format!("{}", i);
        if longest.is_none() { longest = Some(&s); }
    }
    println!("{:?}", longest);
}
