// Partial moves where only the remaining fields are used afterwards.
#[derive(Debug)]
struct Msg {
    header: String,
    body: String,
    id: u32,
}

fn main() {
    let m = Msg { header: "h".into(), body: "b".into(), id: 7 };
    let h = m.header;
    println!("{} {} {}", h, m.body, m.id);
    let b = m.body;
    println!("{} {}", b, m.id);
    let t = (String::from("x"), String::from("y"), 3);
    let (a, _, c) = (t.0, (), t.2);
    println!("{} {} {}", a, t.1, c);
    let mut m2 = Msg { header: "h2".into(), body: "b2".into(), id: 8 };
    let old = m2.body;
    m2.body = format!("{}!", old);
    println!("{:?}", m2);
}
