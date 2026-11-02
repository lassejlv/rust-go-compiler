fn main() {
    let s = "Hello, Wörld!";
    print!("chars:");
    for c in s.chars() {
        print!(" {}", c);
    }
    println!();
    print!("bytes:");
    for b in s.bytes() {
        print!(" {}", b);
    }
    println!();
    println!("len bytes = {}, len chars = {}", s.len(), s.chars().count());
    for (i, c) in s.char_indices() {
        if !c.is_ascii() {
            println!("non-ascii {:?} at byte {}", c, i);
        }
    }
    let rev: String = s.chars().rev().collect();
    println!("reversed = {}", rev);
    println!("nth(4) = {:?}", s.chars().nth(4));
    println!("last = {:?}", s.chars().last());
    let upper_count = s.chars().filter(|c| c.is_uppercase()).count();
    println!("uppercase count = {}", upper_count);
    let bytes = s.as_bytes();
    println!("as_bytes[0..5] = {:?}", &bytes[0..5]);
    let from_bytes = String::from_utf8(vec![82, 117, 115, 116]).unwrap();
    println!("from_utf8 = {}", from_bytes);
    let lossy = String::from_utf8_lossy(&[72, 105]);
    println!("lossy = {}", lossy);
    let digits: u32 = "a1b2c3d4".chars().filter_map(|c| c.to_digit(10)).sum();
    println!("digit sum = {}", digits);
    let mut freq = [0u32; 26];
    for b in "mississippi".bytes() {
        freq[(b - b'a') as usize] += 1;
    }
    for i in 0..26 {
        if freq[i] > 0 {
            print!("{}={} ", (b'a' + i as u8) as char, freq[i]);
        }
    }
    println!();
    let caps: String = "hello world".split(' ').map(|w| {
        let mut cs = w.chars();
        match cs.next() {
            Some(f) => f.to_uppercase().collect::<String>() + cs.as_str(),
            None => String::new(),
        }
    }).collect::<Vec<String>>().join(" ");
    println!("capitalized = {}", caps);
    let emoji = "a🦀b";
    println!("emoji len = {}, chars = {:?}", emoji.len(), emoji.chars().collect::<Vec<char>>());
    println!("is_char_boundary(2) = {}", emoji.is_char_boundary(2));
    let byte_str = b"bytes";
    println!("byte string = {:?}", byte_str);
}
