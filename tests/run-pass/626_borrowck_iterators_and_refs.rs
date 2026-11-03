// Iterator adapters holding borrows that end before mutation.
fn main() {
    let mut v: Vec<i32> = (1..=6).collect();
    let evens: Vec<i32> = v.iter().filter(|x| *x % 2 == 0).copied().collect();
    v.extend(evens.iter().map(|x| x * 10));
    let max = *v.iter().max().unwrap();
    v.push(max + 1);
    let pos = v.iter().position(|&x| x == 4).unwrap();
    v.remove(pos);
    let mut words = vec![String::from("b"), String::from("a")];
    let longest_len = words.iter().map(|w| w.len()).max().unwrap_or(0);
    words.push("c".repeat(longest_len + 1));
    words.sort();
    if let Some(w) = words.iter_mut().find(|w| w.as_str() == "a") {
        w.push('!');
    }
    v.iter_mut().filter(|x| **x > 30).for_each(|x| *x = 0);
    println!("{:?} {:?}", v, words);
}
