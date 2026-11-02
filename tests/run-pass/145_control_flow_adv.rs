fn find_pair(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize)> {
    'outer: for (i, row) in grid.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            if v < 0 {
                continue 'outer;
            }
            if v == target {
                return Some((i, j));
            }
        }
    }
    None
}

fn first_word_len(s: &str) -> usize {
    let Some(w) = s.split_whitespace().next() else {
        return 0;
    };
    w.len()
}

fn parse_kv(s: &str) -> Result<(String, i32), String> {
    let Some((k, v)) = s.split_once('=') else {
        return Err(format!("no '=' in {:?}", s));
    };
    let Ok(n) = v.trim().parse::<i32>() else {
        return Err(format!("bad value {:?}", v));
    };
    Ok((k.trim().to_string(), n))
}

fn classify(a: Option<i32>, b: Result<&str, i32>) -> String {
    if let Some(x) = a.filter(|x| *x > 10) {
        format!("big a {}", x)
    } else if let Ok(s) = b {
        format!("b ok {}", s)
    } else if let (Some(x), Err(e)) = (a, b) {
        format!("a {} with err {}", x, e)
    } else {
        "nothing".to_string()
    }
}

fn sum_until_negative(v: &[i32]) -> i32 {
    v.iter().try_fold(0, |acc, &x| if x < 0 { Err(acc) } else { Ok(acc + x) }).unwrap_or_else(|e| e)
}

fn main() {
    let grid = vec![vec![1, 2, 3], vec![-1, 7, 7], vec![4, 7, 9]];
    println!("{:?}", find_pair(&grid, 7));
    println!("{:?}", find_pair(&grid, 100));

    let mut counter = 0;
    let found = 'search: loop {
        counter += 1;
        let mut inner = 0;
        while inner < 10 {
            inner += 3;
            if counter * inner == 36 {
                break 'search (counter, inner);
            }
        }
        if counter > 50 {
            break (0, 0);
        }
    };
    println!("found {:?} counter {}", found, counter);

    let value = {
        let mut x = 1u64;
        loop {
            x *= 3;
            if x > 1000 {
                break x;
            }
        }
    };
    println!("value {}", value);

    let block_val = 'blk: {
        if value % 2 == 0 {
            break 'blk "even";
        }
        if value > 2000 {
            break 'blk "odd-big";
        }
        "odd-small"
    };
    println!("block {}", block_val);

    let mut stack = vec![1, 2, 3];
    let mut log = vec![];
    while let Some(top) = stack.pop() {
        log.push(top);
        if top == 3 {
            stack.push(10);
            stack.push(20);
        }
    }
    println!("{:?}", log);

    println!("{} {} {}", first_word_len("  hello world"), first_word_len(""), first_word_len("x"));
    for s in ["a = 5", "b=x", "noeq", " key =  -12 "] {
        println!("{:?}", parse_kv(s));
    }
    println!("{}", classify(Some(50), Ok("x")));
    println!("{}", classify(Some(5), Ok("y")));
    println!("{}", classify(Some(5), Err(9)));
    println!("{}", classify(None, Err(1)));

    let results: Vec<i32> = (1..=6)
        .map(|n| {
            let f = |k: i32| -> i32 {
                for d in 2..k {
                    if k % d == 0 {
                        return d;
                    }
                }
                k
            };
            f(n * 7 + 1)
        })
        .collect();
    println!("{:?}", results);
    println!("{} {}", sum_until_negative(&[1, 2, 3, -1, 100]), sum_until_negative(&[5, 5]));

    let mut n = 27u64;
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    println!("collatz steps {}", steps);
}
