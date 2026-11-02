// Slice patterns: fixed-length, prefix/suffix with rest bindings (rest @ ..), nested slice
// patterns, slice patterns on arrays (by value and by ref), on &str slices, and in let/if-let/while-let.
fn classify(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [x] => format!("one({})", x),
        [a, b] if a == b => format!("pair-eq({})", a),
        [a, b] => format!("pair({},{})", a, b),
        [first, .., last] if first == last => format!("bookends({}) len {}", first, v.len()),
        [0, rest @ ..] => format!("zero-led rest={:?}", rest),
        [.., 0] => "zero-tail".into(),
        [first, middle @ .., last] => format!("{} [{}] {}", first, middle.len(), last),
    }
}

fn sum_rec(v: &[i64]) -> i64 {
    match v {
        [] => 0,
        [head, tail @ ..] => head + sum_rec(tail),
    }
}

fn pairs_rev(v: &[u8]) -> Vec<(u8, u8)> {
    let mut out = Vec::new();
    let mut s = v;
    while let [a, b, rest @ ..] = s {
        out.push((*b, *a));
        s = rest;
    }
    if let [leftover] = s {
        out.push((*leftover, 0));
    }
    out
}

fn cmd(args: &[&str]) -> String {
    match args {
        ["help"] | [] => "usage".into(),
        ["get", key] => format!("GET {}", key),
        ["set", key, value] => format!("SET {}={}", key, value),
        ["set", key, values @ ..] => format!("SET {}=[{}]", key, values.join("|")),
        ["del", keys @ ..] if !keys.is_empty() => format!("DEL {} keys", keys.len()),
        [other, ..] => format!("unknown {}", other),
    }
}

fn matrix_kind(m: &[[i32; 2]; 2]) -> &'static str {
    match m {
        [[1, 0], [0, 1]] => "identity",
        [[0, 0], [0, 0]] => "zero",
        [[a, 0], [0, d]] if a == d => "scalar",
        [[_, 0], [0, _]] => "diagonal",
        [[_, _], [0, _]] => "upper",
        _ => "general",
    }
}

fn main() {
    let tests: Vec<Vec<i32>> = vec![
        vec![],
        vec![7],
        vec![2, 2],
        vec![2, 3],
        vec![5, 1, 5],
        vec![0, 1, 2],
        vec![1, 2, 0],
        vec![1, 2, 3, 4, 9],
    ];
    for t in &tests {
        println!("{:?} -> {}", t, classify(t));
    }

    println!("sum {}", sum_rec(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]));
    println!("pairs {:?}", pairs_rev(&[1, 2, 3, 4, 5]));

    for line in ["", "help", "get a", "set a 1", "set a 1 2 3", "del x y", "del", "frob z"] {
        let args: Vec<&str> = line.split_whitespace().collect();
        println!("{:?} => {}", line, cmd(&args));
    }

    let ms = [[[1, 0], [0, 1]], [[0, 0], [0, 0]], [[3, 0], [0, 3]], [[3, 0], [0, 4]], [[1, 2], [0, 4]], [[1, 2], [3, 4]]];
    for m in &ms {
        println!("{:?} {}", m, matrix_kind(m));
    }

    // array by value
    let arr = [10, 20, 30, 40, 50];
    let [a, b, rest @ ..] = arr;
    println!("a={} b={} rest={:?}", a, b, rest);
    let [.., y, z] = arr;
    println!("y={} z={}", y, z);
    if let [first, .., 50] = arr {
        println!("ends with 50, first {}", first);
    }

    // mutable slice pattern
    let mut buf = [1, 2, 3, 4];
    if let [first, .., last] = &mut buf {
        std::mem::swap(first, last);
        *first *= 10;
    }
    println!("buf {:?}", buf);
    match &mut buf[..] {
        [_, mid @ .., _] => {
            for m in mid.iter_mut() {
                *m = -*m;
            }
        }
        _ => {}
    }
    println!("buf {:?}", buf);

    // nested slices
    let grid: Vec<Vec<char>> = vec!["ab".chars().collect(), "c".chars().collect(), vec![]];
    for row in &grid {
        let d = match row.as_slice() {
            [c @ 'a'..='b', ..] => format!("starts low {}", c),
            [c] => format!("single {}", c),
            _ => "other".into(),
        };
        println!("{}", d);
    }
    let words = ["alpha", "beta", "gamma", "delta"];
    let (head, tail) = match &words[..] {
        [h, t @ ..] => (*h, t.len()),
        [] => ("", 0),
    };
    println!("{} {}", head, tail);
    let bytes = "GET /index.html".as_bytes();
    if let [b'G', b'E', b'T', b' ', path @ ..] = bytes {
        println!("path = {}", std::str::from_utf8(path).unwrap());
    }
}
