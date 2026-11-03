// dynamic programming: LCS with reconstruction, edit distance, knapsack, coin change, LIS, matrix chain, partitions
fn lcs(a: &str, b: &str) -> String {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            dp[i][j] = if a[i - 1] == b[j - 1] { dp[i - 1][j - 1] + 1 } else { dp[i - 1][j].max(dp[i][j - 1]) };
        }
    }
    let (mut i, mut j) = (a.len(), b.len());
    let mut out = vec![];
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            out.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    out.iter().rev().collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for j in 0..b.len() {
            cur[j + 1] = (prev[j] + (ca != b[j]) as usize).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn knapsack(items: &[(usize, u64)], cap: usize) -> (u64, Vec<usize>) {
    let n = items.len();
    let mut dp = vec![vec![0u64; cap + 1]; n + 1];
    for i in 1..=n {
        let (w, v) = items[i - 1];
        for c in 0..=cap {
            dp[i][c] = dp[i - 1][c];
            if c >= w {
                dp[i][c] = dp[i][c].max(dp[i - 1][c - w] + v);
            }
        }
    }
    let mut chosen = vec![];
    let mut c = cap;
    for i in (1..=n).rev() {
        if dp[i][c] != dp[i - 1][c] {
            chosen.push(i - 1);
            c -= items[i - 1].0;
        }
    }
    chosen.reverse();
    (dp[n][cap], chosen)
}

fn coin_ways(coins: &[usize], amt: usize) -> u64 {
    let mut ways = vec![0u64; amt + 1];
    ways[0] = 1;
    for &c in coins {
        for a in c..=amt {
            ways[a] += ways[a - c];
        }
    }
    ways[amt]
}

fn min_coins(coins: &[usize], amt: usize) -> Option<usize> {
    let mut dp = vec![None; amt + 1];
    dp[0] = Some(0);
    for a in 1..=amt {
        dp[a] = coins.iter().filter(|&&c| c <= a).filter_map(|&c| dp[a - c].map(|k: usize| k + 1)).min();
    }
    dp[amt]
}

fn lis(v: &[i32]) -> Vec<i32> {
    let mut tails: Vec<usize> = vec![];
    let mut parent = vec![None; v.len()];
    for i in 0..v.len() {
        let pos = tails.partition_point(|&t| v[t] < v[i]);
        if pos > 0 {
            parent[i] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(i);
        } else {
            tails[pos] = i;
        }
    }
    let mut out = vec![];
    let mut cur = tails.last().copied();
    while let Some(i) = cur {
        out.push(v[i]);
        cur = parent[i];
    }
    out.reverse();
    out
}

fn matrix_chain(d: &[u64]) -> (u64, String) {
    let n = d.len() - 1;
    let mut m = vec![vec![0u64; n]; n];
    let mut s = vec![vec![0usize; n]; n];
    for len in 2..=n {
        for i in 0..=n - len {
            let j = i + len - 1;
            m[i][j] = u64::MAX;
            for k in i..j {
                let c = m[i][k] + m[k + 1][j] + d[i] * d[k + 1] * d[j + 1];
                if c < m[i][j] {
                    m[i][j] = c;
                    s[i][j] = k;
                }
            }
        }
    }
    fn paren(s: &[Vec<usize>], i: usize, j: usize) -> String {
        if i == j { format!("A{}", i + 1) } else { format!("({}{})", paren(s, i, s[i][j]), paren(s, s[i][j] + 1, j)) }
    }
    (m[0][n - 1], paren(&s, 0, n - 1))
}

fn partitions(n: usize) -> u64 {
    let mut p = vec![0u64; n + 1];
    p[0] = 1;
    for k in 1..=n {
        for i in k..=n {
            p[i] += p[i - k];
        }
    }
    p[n]
}

fn main() {
    println!("{:?}", lcs("AGGTABQ", "GXTXAYBQ"));
    println!("{:?}", lcs("日本語テキスト", "本テスト"));
    for (a, b) in [("kitten", "sitting"), ("", "abc"), ("flaw", "lawn"), ("intention", "execution")] {
        println!("{} -> {}: {}", a, b, edit_distance(a, b));
    }
    println!("{:?}", knapsack(&[(12, 4), (2, 2), (1, 1), (1, 2), (4, 10)], 15));
    println!("{} {}", coin_ways(&[1, 2, 5], 11), coin_ways(&[2, 5, 3, 6], 10));
    println!("{} {}", coin_ways(&[1, 5, 10, 25, 50, 100], 300), coin_ways(&[7], 3));
    println!("{:?} {:?} {:?}", min_coins(&[1, 2, 5], 11), min_coins(&[2], 3), min_coins(&[9, 6, 5, 1], 11));
    println!("{:?}", lis(&[10, 9, 2, 5, 3, 7, 101, 18, 4, 19]));
    println!("{:?}", lis(&[3, 2, 1]));
    println!("{:?}", matrix_chain(&[40, 20, 30, 10, 30]));
    println!("{:?}", matrix_chain(&[10, 20, 30, 40, 30]));
    println!("{:?}", (1..=10).map(partitions).collect::<Vec<_>>());
    println!("{}", partitions(100));
    let tri = vec![vec![2], vec![3, 4], vec![6, 5, 7], vec![4, 1, 8, 3]];
    let best = tri.iter().rev().skip(1).fold(tri.last().unwrap().clone(), |below, row| {
        row.iter().enumerate().map(|(i, v)| v + below[i].min(below[i + 1])).collect()
    });
    println!("{:?}", best);
}
