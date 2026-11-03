// Moving in one match arm while using the value in another.
fn main() {
    let items = vec![String::from("a"), String::from("b")];
    let mut sink = Vec::new();
    for flag in [true, false] {
        let s = String::from(if flag { "keep" } else { "drop" });
        match flag {
            true => sink.push(s),
            false => println!("still have {}", s),
        }
    }
    let n = items.len();
    let result = if n > 1 { items } else { vec![format!("{}", items.len())] };
    println!("{:?} {:?}", sink, result);
    let o = Some(String::from("payload"));
    let msg = match o {
        Some(ref s) if s.len() > 100 => s.clone(),
        Some(s) => s,
        None => String::from("none"),
    };
    println!("{}", msg);
}
