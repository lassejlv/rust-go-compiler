fn solve(n: usize, row: usize, cols: &mut Vec<usize>, used_col: &mut Vec<bool>, d1: &mut Vec<bool>, d2: &mut Vec<bool>, out: &mut Vec<Vec<usize>>) {
    if row == n {
        out.push(cols.clone());
        return;
    }
    for c in 0..n {
        let a = row + c;
        let b = row + n - 1 - c;
        if used_col[c] || d1[a] || d2[b] {
            continue;
        }
        used_col[c] = true;
        d1[a] = true;
        d2[b] = true;
        cols.push(c);
        solve(n, row + 1, cols, used_col, d1, d2, out);
        cols.pop();
        used_col[c] = false;
        d1[a] = false;
        d2[b] = false;
    }
}

fn all_solutions(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    solve(n, 0, &mut Vec::new(), &mut vec![false; n], &mut vec![false; 2 * n], &mut vec![false; 2 * n], &mut out);
    out
}

fn count_bits(n: u32, row: u32, cols: u32, d1: u32, d2: u32) -> u64 {
    if row == n {
        return 1;
    }
    let full = (1u32 << n) - 1;
    let mut avail = full & !(cols | d1 | d2);
    let mut total = 0;
    while avail != 0 {
        let bit = avail & avail.wrapping_neg();
        avail &= avail - 1;
        total += count_bits(n, row + 1, cols | bit, (d1 | bit) << 1, (d2 | bit) >> 1);
    }
    total
}

fn render(sol: &[usize]) {
    let n = sol.len();
    for &c in sol {
        let line: String = (0..n).map(|i| if i == c { 'Q' } else { '.' }).collect();
        println!("  {}", line);
    }
}

fn main() {
    for n in 1..=8 {
        let sols = all_solutions(n);
        println!("n={} solutions={}", n, sols.len());
    }
    let eight = all_solutions(8);
    println!("first 8-queens solution {:?}:", eight[0]);
    render(&eight[0]);
    println!("last 8-queens solution {:?}:", eight[eight.len() - 1]);
    render(&eight[eight.len() - 1]);
    let symmetric = eight.iter().filter(|s| {
        let rev: Vec<usize> = s.iter().rev().cloned().collect();
        eight.contains(&rev)
    }).count();
    println!("solutions whose vertical mirror is also listed: {}", symmetric);
    for n in [4, 6, 8, 10] {
        println!("bitmask count n={}: {}", n, count_bits(n, 0, 0, 0, 0));
    }
}
