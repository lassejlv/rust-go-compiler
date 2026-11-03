fn find_pair(grid: &[Vec<i32>], target: i32) -> Option<(usize, usize, usize, usize)> {
    let res = 'outer: {
        for (i, row) in grid.iter().enumerate() {
            for (j, &a) in row.iter().enumerate() {
                for (k, row2) in grid.iter().enumerate().skip(i) {
                    for (l, &b) in row2.iter().enumerate() {
                        if (k, l) <= (i, j) { continue; }
                        if a + b == target { break 'outer Some((i, j, k, l)); }
                    }
                }
            }
        }
        None
    };
    res
}

fn collatz_len(mut n: u64) -> u32 {
    let mut steps = 0;
    while n != 1 { n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 }; steps += 1; }
    steps
}

fn classify(n: i32) -> &'static str {
    let x = n;
    let x = x * 2;
    let x = if x > 10 { "big" } else { let x = x - 1; if x < 0 { "neg" } else { "small" } };
    x
}

fn main() {
    let grid = vec![vec![1, 5, 9], vec![2, 8, 3], vec![7, 4, 6]];
    println!("{:?} {:?} {:?}", find_pair(&grid, 17), find_pair(&grid, 3), find_pair(&grid, 100));

    let mut count = 0;
    'a: for i in 0..10 {
        'b: for j in 0..10 {
            if j > i { continue 'a; }
            if (i + j) % 7 == 6 { break 'b; }
            if i * j > 30 { break 'a; }
            count += 1;
        }
    }
    println!("count {}", count);

    let mut n = 0;
    let found = loop {
        n += 1;
        if n % 7 == 0 && n % 5 == 3 { break n * 10; }
        if n > 1000 { break -1; }
    };
    println!("found {}", found);

    let best = (1..30u64).map(|n| (collatz_len(n), n)).max().unwrap();
    println!("collatz best {:?}", best);

    let v: Vec<i32> = vec![5, -3, 8, 0, 12, -7];
    let mut acc = Vec::new();
    let mut it = v.iter();
    while let Some(&x) = it.next() {
        let tag = match x {
            i32::MIN..=-1 => { if x < -5 { "very neg" } else { "neg" } }
            0 => continue,
            1..=9 => "digit",
            _ => { acc.push(format!("big:{}", x)); continue; }
        };
        acc.push(format!("{}:{}", tag, x));
    }
    println!("{:?}", acc);

    let x = 5;
    {
        let x = "shadow";
        println!("inner {}", x);
        let x = x.len();
        println!("inner2 {}", x);
    }
    let x = x as f64 * 1.5;
    println!("outer {}", x);
    for x in [1, 6, -4] { println!("{} -> {}", x, classify(x)); }

    let label_val = 'blk: {
        if v.len() < 3 { break 'blk "short"; }
        if v.contains(&0) { break 'blk "has zero"; }
        "plain"
    };
    println!("{}", label_val);

    let mut stack = vec![1];
    let mut visited = 0;
    while let Some(top) = stack.pop() {
        visited += 1;
        if top < 20 { stack.push(top * 2); if top % 3 == 1 { stack.push(top * 3); } }
    }
    println!("visited {}", visited);

    let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    let diag: i32 = (0..3).map(|i| matrix[i][i]).sum();
    let anti: i32 = (0..3).map(|i| matrix[i][2 - i]).sum();
    let mut spiral = vec![];
    let (mut top, mut bot, mut left, mut right) = (0i32, 2i32, 0i32, 2i32);
    while top <= bot && left <= right {
        for j in left..=right { spiral.push(matrix[top as usize][j as usize]); }
        top += 1;
        for i in top..=bot { spiral.push(matrix[i as usize][right as usize]); }
        right -= 1;
        if top <= bot { for j in (left..=right).rev() { spiral.push(matrix[bot as usize][j as usize]); } bot -= 1; }
        if left <= right { for i in (top..=bot).rev() { spiral.push(matrix[i as usize][left as usize]); } left += 1; }
    }
    println!("diag {} anti {} spiral {:?}", diag, anti, spiral);

    let r = (|| -> Result<i32, String> {
        let a: i32 = "12".parse().map_err(|_| "a")?;
        let b: i32 = "x".parse().or_else(|_| if a > 10 { Ok(1) } else { Err("b".to_string()) })?;
        Ok(a + b)
    })();
    println!("{:?}", r);
    let mut fizz = String::new();
    for i in 1..=15 {
        let s = match (i % 3, i % 5) { (0, 0) => "FizzBuzz".to_string(), (0, _) => "Fizz".into(), (_, 0) => "Buzz".into(), _ => i.to_string() };
        fizz += &s;
        if i != 15 { fizz.push(','); }
    }
    println!("{}", fizz);
    let t = if let Some(x) = v.iter().find(|&&x| x > 10) { *x } else if let Some(y) = v.first() { *y } else { 0 };
    println!("{}", t);
    let mut ret_count = 0;
    let r2 = loop {
        ret_count += 1;
        match ret_count { 1 | 2 => continue, 3 => {}, _ => break "done" }
        if ret_count == 3 { ret_count += 10; }
    };
    println!("{} {}", r2, ret_count);
}
