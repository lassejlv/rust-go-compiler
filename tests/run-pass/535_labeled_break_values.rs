// labeled breaks/continues, loop-break-value, labeled blocks, nested loops
fn find_pair(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize)> {
    let r = 'outer: loop {
        for (i, row) in grid.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if v == target {
                    break 'outer Some((i, j));
                }
            }
        }
        break None;
    };
    r
}

fn classify(n: i32) -> &'static str {
    let kind = 'blk: {
        if n < 0 {
            break 'blk "negative";
        }
        if n == 0 {
            break 'blk "zero";
        }
        if n % 2 == 0 {
            break 'blk "even";
        }
        "odd"
    };
    kind
}

fn primes_below(n: u32) -> Vec<u32> {
    let mut out = vec![];
    'cand: for c in 2..n {
        for p in &out {
            if p * p > c {
                break;
            }
            if c % p == 0 {
                continue 'cand;
            }
        }
        out.push(c);
    }
    out
}

fn main() {
    let grid = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
    println!("{:?} {:?}", find_pair(&grid, 6), find_pair(&grid, 10));
    for n in [-3, 0, 4, 7] {
        print!("{} ", classify(n));
    }
    println!();
    println!("{:?}", primes_below(50));
    let mut counter = 0;
    let v = loop {
        counter += 3;
        if counter > 20 {
            break counter * 2;
        }
    };
    println!("{}", v);
    let mut log = vec![];
    'a: for i in 0..4 {
        'b: for j in 0..4 {
            if j > i {
                continue 'a;
            }
            if i + j == 5 {
                break 'a;
            }
            if (i * j) % 2 == 1 {
                continue 'b;
            }
            log.push((i, j));
        }
    }
    println!("{:?}", log);
    let mut x = 0u64;
    let steps = 'w: {
        let mut s = 0;
        while x < 1000 {
            x = x * 2 + 1;
            s += 1;
            if s > 100 {
                break 'w -1;
            }
        }
        s
    };
    println!("{} {}", steps, x);
    let result = 'search: {
        for word in ["apple", "banana", "cherry"] {
            for (i, c) in word.chars().enumerate() {
                if c == 'r' {
                    break 'search Some((word, i));
                }
            }
        }
        None
    };
    println!("{:?}", result);
    let mut it = 0;
    let nested = 'outer: loop {
        let inner = loop {
            it += 1;
            if it % 5 == 0 {
                break it;
            }
            if it > 12 {
                break 'outer it * 100;
            }
        };
        if inner > 10 {
            break inner + 1000;
        }
    };
    println!("{} {}", nested, it);
    let mut total = 0;
    'rows: while total < 100 {
        for k in 1.. {
            total += k;
            if k == 4 {
                continue 'rows;
            }
        }
    }
    println!("{}", total);
    let collatz_max = (1..30u64).map(|start| {
        let mut n = start;
        let mut steps = 0;
        let peak = loop {
            if n == 1 {
                break steps;
            }
            n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
            steps += 1;
        };
        (peak, start)
    }).max();
    println!("{:?}", collatz_max);
    let found = 'f: {
        let data = [3, 8, 12, 7];
        let mut i = 0;
        while i < data.len() {
            if data[i] > 10 {
                break 'f Ok(i);
            }
            i += 1;
        }
        Err("none")
    };
    println!("{:?}", found);
    let s = loop {
        break String::from("from loop");
    };
    println!("{}", s);
    let opt = 'o: {
        let v: Vec<i32> = Vec::new();
        let Some(first) = v.first() else { break 'o -1 };
        *first
    };
    println!("{}", opt);
}
