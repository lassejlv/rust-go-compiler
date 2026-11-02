fn describe(s: &[i32]) -> String {
    match s {
        [] => "empty".to_string(),
        [x] => format!("one: {}", x),
        [a, b] => format!("two: {} {}", a, b),
        [first, .., last] if first == last => format!("bookends {}", first),
        [first, middle @ .., last] => format!("first {} middle {:?} last {}", first, middle, last),
    }
}

fn sum_rec(s: &[i32]) -> i32 {
    match s {
        [] => 0,
        [head, tail @ ..] => head + sum_rec(tail),
    }
}

fn command(args: &[&str]) -> String {
    match args {
        ["get", key] => format!("GET {}", key),
        ["set", key, value] => format!("SET {}={}", key, value),
        ["del", keys @ ..] if !keys.is_empty() => format!("DEL {}", keys.join(",")),
        [cmd, ..] => format!("unknown {}", cmd),
        [] => "no command".to_string(),
    }
}

fn main() {
    for s in [&[][..], &[1], &[1, 2], &[4, 5, 4], &[1, 2, 3, 4, 5]] {
        println!("{:?} -> {}", s, describe(s));
    }
    println!("sum_rec = {}", sum_rec(&[1, 2, 3, 4, 5, 6]));
    for c in [vec!["get", "a"], vec!["set", "b", "2"], vec!["del", "x", "y"], vec!["del"], vec!["ping"], vec![]] {
        println!("{:?} -> {}", c, command(&c));
    }
    let arr = [10, 20, 30, 40];
    let [a, b, rest @ ..] = arr;
    println!("a={} b={} rest={:?}", a, b, rest);
    if let [.., penult, _] = arr {
        println!("penultimate = {}", penult);
    }

    let grid = [[1, 2, 3], [4, 42, 6], [7, 8, 9]];
    let mut found = None;
    'outer: for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v == 42 {
                found = Some((r, c));
                break 'outer;
            }
        }
    }
    println!("found 42 at {:?}", found);

    let mut count = 0;
    'rows: for i in 0..5 {
        for j in 0..5 {
            if j > i {
                continue 'rows;
            }
            if i + j == 6 {
                break 'rows;
            }
            count += 1;
        }
    }
    println!("count = {}", count);

    let result = 'search: loop {
        let mut n = 1u64;
        loop {
            n += 1;
            if n * n > 500 {
                break 'search n;
            }
        }
    };
    println!("first n with n^2 > 500: {}", result);

    let value = 'block: {
        if arr[0] > 5 {
            break 'block "big first";
        }
        "small first"
    };
    println!("labeled block: {}", value);

    let mut pairs = Vec::new();
    'a: for x in 1..10 {
        'b: for y in x..10 {
            if (x * y) % 7 == 0 {
                pairs.push((x, y));
                continue 'a;
            }
            if y - x > 3 {
                break 'b;
            }
        }
    }
    println!("pairs = {:?}", pairs);
}
