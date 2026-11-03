// Storing references to longer-lived data in a vector; mutation after use.
fn main() {
    let words = vec![String::from("pear"), String::from("fig"), String::from("apple")];
    let mut refs: Vec<&String> = Vec::new();
    for w in &words {
        if w.len() > 3 {
            refs.push(w);
        }
    }
    refs.sort();
    println!("{:?}", refs);
    let mut owned = words;
    owned.push(String::from("kiwi"));
    let mut by_len: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    by_len.sort_by_key(|s| s.len());
    let shortest = by_len[0].to_string();
    owned.retain(|s| *s != shortest);
    println!("{} {:?}", shortest, owned);
}
