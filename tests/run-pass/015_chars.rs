fn main() {
    let c = 'a';
    println!("c = {}", c);
    println!("c as u8 = {}", c as u8);
    println!("c as u32 = {}", c as u32);
    println!("char::from(66) = {}", char::from(66u8));
    println!("next char = {}", (c as u8 + 1) as char);
    println!("is_alphabetic: {} {}", 'z'.is_alphabetic(), '3'.is_alphabetic());
    println!("is_numeric: {} {}", '7'.is_numeric(), 'x'.is_numeric());
    println!("is_ascii_digit: {}", '9'.is_ascii_digit());
    println!("is_alphanumeric: {} {}", 'q'.is_alphanumeric(), '!'.is_alphanumeric());
    println!("is_whitespace: {} {}", ' '.is_whitespace(), '\n'.is_whitespace());
    println!("is_uppercase: {} {}", 'Q'.is_uppercase(), 'q'.is_uppercase());
    println!("is_lowercase: {}", 'q'.is_lowercase());
    println!("to_ascii_uppercase: {}", 'm'.to_ascii_uppercase());
    println!("to_ascii_lowercase: {}", 'M'.to_ascii_lowercase());
    println!("to_digit: {:?} {:?}", '7'.to_digit(10), 'f'.to_digit(16));
    println!("from_digit: {:?}", std::char::from_digit(5, 10));
    println!("is_ascii_punctuation: {}", '?'.is_ascii_punctuation());
    for ch in 'a'..='e' {
        print!("{}", ch.to_ascii_uppercase());
    }
    println!();
    let mut code = 0u32;
    for ch in "Rust".chars() {
        code = code * 31 + ch as u32;
    }
    println!("hash = {}", code);
    let digit = '8';
    let value = digit as u8 - b'0';
    println!("digit value = {}", value);
    println!("byte literal: {}", b'A');
    println!("char compare: {} {}", 'a' < 'b', 'Z' < 'a');
    let e = 'é';
    println!("len_utf8: {} {}", 'a'.len_utf8(), e.len_utf8());
    println!("unicode char: {} {}", e, e as u32);
    println!("escaped: {:?} {:?} {:?}", '\n', '\'', 'x');
}
