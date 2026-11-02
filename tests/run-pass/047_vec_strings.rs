fn main() {
    let mut names: Vec<String> = Vec::new();
    for n in ["carol", "alice", "bob", "alice", "dave", "bob"] {
        names.push(n.to_string());
    }
    println!("names: {:?}", names);
    names.sort();
    names.dedup();
    println!("unique sorted: {:?}", names);
    let upper: Vec<String> = names.iter().map(|n| n.to_uppercase()).collect();
    println!("upper: {}", upper.join(", "));
    let lens: Vec<usize> = names.iter().map(|n| n.len()).collect();
    println!("lens: {:?}", lens);
    let long: Vec<&String> = names.iter().filter(|n| n.len() > 3).collect();
    println!("long: {:?}", long);
    let initials: String = names.iter().map(|n| n.chars().next().unwrap()).collect();
    println!("initials: {}", initials);
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    println!("refs contains bob: {}", refs.contains(&"bob"));
    let pos = names.iter().position(|n| n == "carol");
    println!("carol at {:?}", pos);
    let mut sentence = Vec::new();
    for w in "the cat sat on the mat".split(' ') {
        if !sentence.contains(&w) {
            sentence.push(w);
        }
    }
    println!("unique words in order: {:?}", sentence);
    let numbered: Vec<String> = names.iter().enumerate().map(|(i, n)| format!("{}. {}", i + 1, n)).collect();
    for line in &numbered {
        println!("{}", line);
    }
    let total_chars: usize = names.iter().map(String::len).sum();
    println!("total chars: {}", total_chars);
    let mut owned = names.clone();
    owned.retain(|n| !n.starts_with('a'));
    owned.insert(0, String::from("zed"));
    println!("owned: {:?}, original: {:?}", owned, names);
    let longest = names.iter().max_by_key(|n| n.len()).unwrap();
    println!("longest: {}", longest);
    let csv_line = names.iter().map(|n| format!("\"{}\"", n)).collect::<Vec<_>>().join(",");
    println!("csv: {}", csv_line);
}
