fn is_palindrome(s: &str) -> bool {
    let mut it = s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_lowercase());
    loop {
        match (it.next(), it.next_back()) {
            (Some(a), Some(b)) => {
                if a != b {
                    return false;
                }
            }
            _ => return true,
        }
    }
}

fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let mut it = v.iter();
    println!("next {:?}, next_back {:?}", it.next(), it.next_back());
    println!("next {:?}, next_back {:?}", it.next(), it.next_back());
    println!("remaining {:?}", it.collect::<Vec<_>>());
    let mut r = 1..=10;
    println!("range next_back {:?} {:?}", r.next_back(), r.next());
    println!("range rest count {}", r.count());
    println!("rfind even: {:?}", v.iter().rfind(|&&x| x % 2 == 0));
    println!("rev skip: {:?}", v.iter().rev().skip(1).step_by(2).collect::<Vec<_>>());
    println!("rev chain: {:?}", (1..3).chain(7..9).rev().collect::<Vec<_>>());
    println!("rev zip: {:?}", v.iter().zip(v.iter().rev()).map(|(a, b)| a * b).collect::<Vec<_>>());
    println!("rev map enumerate: {:?}", v.iter().map(|x| x * x).rev().enumerate().take(3).collect::<Vec<_>>());
    println!("nth_back(1): {:?}", v.iter().nth_back(1));
    println!("rfold: {}", v.iter().rfold(String::new(), |acc, x| acc + &x.to_string()));
    println!("len: {}, exact size after skip: {}", v.iter().len(), v.iter().skip(2).len());
    for s in ["racecar", "A man, a plan, a canal: Panama", "hello", "", "ab"] {
        println!("{:?} palindrome? {}", s, is_palindrome(s));
    }
    let word = "stressed";
    println!("{} reversed is {}", word, word.chars().rev().collect::<String>());
    let bytes_rev: Vec<u8> = "abc".bytes().rev().collect();
    println!("bytes rev: {:?}", bytes_rev);
    let last_two: Vec<&str> = "a/b/c/d".rsplit('/').take(2).collect();
    println!("rsplit take 2: {:?}", last_two);
    let mut pos = 0;
    let digits: Vec<u32> = "9876".chars().rev().map(|c| c.to_digit(10).unwrap()).collect();
    let mut value = 0;
    for d in &digits {
        value += d * 10u32.pow(pos);
        pos += 1;
    }
    println!("reconstructed: {}", value);
}
