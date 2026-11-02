fn describe(n: i32) -> &'static str {
    match n {
        0 => "zero",
        1 | 2 | 3 => "small",
        4..=9 => "medium",
        10..=99 => "large",
        i32::MIN..=-1 => "negative",
        _ => "huge",
    }
}

fn grade(score: u32) -> char {
    match score {
        90..=100 => 'A',
        80..=89 => 'B',
        70..=79 => 'C',
        60..=69 => 'D',
        _ => 'F',
    }
}

fn fizzbuzz(n: u32) -> String {
    match (n % 3, n % 5) {
        (0, 0) => "FizzBuzz".to_string(),
        (0, _) => "Fizz".to_string(),
        (_, 0) => "Buzz".to_string(),
        _ => n.to_string(),
    }
}

fn sign_desc(x: i32, y: i32) -> &'static str {
    match (x, y) {
        (0, 0) => "origin",
        (x, 0) if x > 0 => "positive x-axis",
        (_, 0) => "negative x-axis",
        (0, _) => "y-axis",
        (x, y) if x == y => "diagonal",
        (x, y) if x > 0 && y > 0 => "quadrant I",
        _ => "elsewhere",
    }
}

fn char_kind(c: char) -> &'static str {
    match c {
        'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
        'a'..='z' => "consonant",
        '0'..='9' => "digit",
        _ if c.is_whitespace() => "space",
        _ => "other",
    }
}

fn main() {
    for n in [0, 2, 7, 42, -5, 1000] {
        println!("{} -> {}", n, describe(n));
    }
    for s in [95, 85, 72, 64, 10] {
        print!("{}{} ", s, grade(s));
    }
    println!();
    for i in 1..=15 {
        print!("{} ", fizzbuzz(i));
    }
    println!();
    for p in [(0, 0), (5, 0), (-2, 0), (0, 3), (4, 4), (1, 2), (-1, 5)] {
        println!("{:?} -> {}", p, sign_desc(p.0, p.1));
    }
    for c in "ax9 ?".chars() {
        println!("{:?} -> {}", c, char_kind(c));
    }
    let b = true;
    let v = match b {
        true => 1,
        false => 0,
    };
    println!("bool match = {}", v);
}
