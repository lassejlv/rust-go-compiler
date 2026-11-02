use std::fmt::Write;

fn main() {
    let mut s = String::from("hello");
    s.insert(0, '>');
    s.insert_str(1, "> ");
    println!("{}", s);
    let removed = s.remove(0);
    println!("removed {:?} -> {}", removed, s);
    let popped = s.pop();
    println!("popped {:?} -> {}", popped, s);
    s.truncate(4);
    println!("truncated -> {}", s);
    s.clear();
    println!("cleared -> '{}' {}", s, s.is_empty());
    let mut t = String::from("a1b2c3d4");
    t.retain(|c| c.is_alphabetic());
    println!("retain -> {}", t);
    let from_chars: String = vec!['r', 'u', 's', 't'].into_iter().collect();
    println!("from chars -> {}", from_chars);
    let from_strs: String = vec!["ab", "cd", "ef"].concat();
    println!("concat -> {}", from_strs);
    let joined = vec!["x", "y", "z"].join(", ");
    println!("join -> {}", joined);
    let mut out = String::new();
    for i in 0..5 {
        write!(out, "{}-", i).unwrap();
    }
    writeln!(out, "end").unwrap();
    print!("{}", out);
    let mut sq = String::new();
    for i in 1..=4 {
        if !sq.is_empty() {
            sq.push_str(" + ");
        }
        sq += &format!("{}^2", i);
    }
    println!("{} = {}", sq, (1..=4).map(|i| i * i).sum::<i32>());
    let ch = 'Z';
    let cs = ch.to_string() + "ebra";
    println!("{}", cs);
    let mut drained = String::from("hello world");
    let d: String = drained.drain(..6).collect();
    println!("drained '{}' rest '{}'", d, drained);
    let mut rs = String::from("abc");
    rs.replace_range(1..2, "XYZ");
    println!("replace_range -> {}", rs);
    let split_off = {
        let mut a = String::from("headtail");
        let b = a.split_off(4);
        format!("{}|{}", a, b)
    };
    println!("split_off -> {}", split_off);
    let mut extended = String::from("ext");
    extended.extend(['e', 'n', 'd']);
    extended.extend(vec!["ed", "!"]);
    println!("extend -> {}", extended);
    let line = "=".repeat(10);
    println!("{}", line);
}
