// Temporaries that live until the end of the statement / let with extension.
fn make() -> Vec<String> {
    vec![String::from("b"), String::from("a")]
}

fn main() {
    let n = make().iter().map(|s| s.len()).sum::<usize>();
    let first = make()[0].clone();
    let r = &make();
    let joined = String::from("x y z").split(' ').collect::<Vec<_>>().join("-");
    let s: &str = &String::from("extended");
    let t = &mut String::from("mutable temp");
    t.push('!');
    println!("{} {} {:?} {} {} {}", n, first, r, joined, s, t);
    match make().pop() {
        Some(x) => println!("{}", x),
        None => {}
    }
    let len = { let tmp = String::from("scoped"); tmp.len() };
    println!("{}", len);
}
