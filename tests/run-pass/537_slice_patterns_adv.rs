// slice patterns: [first, .., last], [x, rest @ ..], nested, with refs, in fn params, on arrays and Vec
fn sum(v: &[i32]) -> i32 {
    match v {
        [] => 0,
        [x, rest @ ..] => x + sum(rest),
    }
}

fn describe(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [one] => format!("one({})", one),
        [a, b] => format!("two({},{})", a, b),
        [first, .., last] if first == last => format!("bookends({})", first),
        [first, middle @ .., last] => format!("{}..{} with {} inside (sum {})", first, last, middle.len(), sum(middle)),
    }
}

fn palindrome<T: PartialEq>(v: &[T]) -> bool {
    match v {
        [] | [_] => true,
        [a, inner @ .., b] => a == b && palindrome(inner),
    }
}

fn parse_cmd(args: &[&str]) -> String {
    match args {
        ["get", key] => format!("GET {}", key),
        ["set", key, value] => format!("SET {}={}", key, value),
        ["del", keys @ ..] if !keys.is_empty() => format!("DEL {}", keys.join(",")),
        ["mv", .., dst] if args.len() > 2 => format!("MOVE {} files to {}", args.len() - 2, dst),
        [cmd, ..] => format!("unknown {}", cmd),
        [] => "nothing".into(),
    }
}

fn merge(a: &[i32], b: &[i32]) -> Vec<i32> {
    match (a, b) {
        ([], rest) | (rest, []) => rest.to_vec(),
        ([x, xs @ ..], [y, ys @ ..]) => {
            if x <= y {
                let mut r = vec![*x];
                r.extend(merge(xs, b));
                r
            } else {
                let mut r = vec![*y];
                r.extend(merge(a, ys));
                r
            }
        }
    }
}

fn first_two([a, b, ..]: [u8; 5]) -> u16 {
    (a as u16) << 8 | b as u16
}

fn windows_match(v: &[char]) -> usize {
    v.windows(3).filter(|w| matches!(w, [a, _, c] if a == c)).count()
}

#[derive(Debug)]
struct P {
    name: String,
    tags: Vec<String>,
}

fn main() {
    println!("{}", sum(&[1, 2, 3, 4]));
    for v in [&[][..], &[5], &[1, 2], &[7, 3, 7], &[1, 2, 3, 4, 5]] {
        println!("{}", describe(v));
    }
    println!("{} {} {}", palindrome(&[1, 2, 1]), palindrome(&['a', 'b']), palindrome::<u8>(&[]));
    println!("{}", palindrome(&"racecar".chars().collect::<Vec<_>>()));
    for cmd in ["get k", "set a 1", "del x y z", "del", "mv a b c /tmp", "ls -la", ""] {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        println!("{:?} -> {}", cmd, parse_cmd(&parts));
    }
    println!("{:?}", merge(&[1, 4, 6, 9], &[2, 3, 7]));
    println!("{:#06x}", first_two([0xab, 0xcd, 0, 0, 0]));
    println!("{}", windows_match(&"abacadaea".chars().collect::<Vec<_>>()));
    let arr = [10, 20, 30, 40, 50];
    let [a, b, rest @ ..] = arr;
    println!("{} {} {:?}", a, b, rest);
    let [.., y, z] = arr;
    println!("{} {}", y, z);
    if let [_, mid @ .., _] = &arr[..] {
        println!("mid {:?}", mid);
    }
    let mut m = [1, 2, 3, 4];
    if let [first, .., last] = &mut m {
        std::mem::swap(first, last);
    }
    println!("{:?}", m);
    if let [head, tail @ ..] = &mut m[..] {
        *head *= 10;
        for t in tail.iter_mut() {
            *t += 100;
        }
    }
    println!("{:?}", m);
    let people = vec![
        P { name: "a".into(), tags: vec!["x".into()] },
        P { name: "b".into(), tags: vec![] },
        P { name: "c".into(), tags: vec!["y".into(), "z".into()] },
    ];
    match people.as_slice() {
        [P { name, tags }, .., P { tags: last_tags, .. }] => {
            println!("{} {:?} last_tags={:?}", name, tags, last_tags);
        }
        _ => {}
    }
    for p in &people {
        let desc = match p.tags.as_slice() {
            [] => "untagged".to_string(),
            [only] => format!("tag {}", only),
            [first, others @ ..] => format!("tag {} + {}", first, others.len()),
        };
        println!("{} {}", p.name, desc);
    }
    let v = vec![Some(1), None, Some(3)];
    match &v[..] {
        [Some(a), None, Some(b)] => println!("pattern {}", a + b),
        _ => println!("no"),
    }
    let bytes = b"GET /index.html HTTP/1.1";
    match bytes {
        [b'G', b'E', b'T', b' ', rest @ ..] => println!("GET path bytes={}", rest.len()),
        _ => println!("other"),
    }
    let s: &[&str] = &["x"; 3];
    if let [.., "x"] = s {
        println!("ends with x");
    }
    let nested = [[1, 2], [3, 4], [5, 6]];
    let [[a1, _], .., [_, z1]] = nested;
    println!("{} {}", a1, z1);
    let strings = vec![String::from("p"), String::from("q")];
    if let [ref first, ..] = strings[..] {
        println!("{} still {}", first, strings.len());
    }
}
