// Accumulating into a &mut parameter inside nested loops and recursion.
fn collect_paths(n: u32, path: &mut Vec<u32>, out: &mut Vec<Vec<u32>>) {
    if n == 0 {
        out.push(path.clone());
        return;
    }
    for step in 1..=2 {
        if step <= n {
            path.push(step);
            collect_paths(n - step, path, out);
            path.pop();
        }
    }
}

fn main() {
    let mut out = Vec::new();
    let mut path = Vec::new();
    collect_paths(4, &mut path, &mut out);
    println!("{} {:?}", out.len(), out);
    let mut grid = vec![vec![0u32; 4]; 3];
    for r in 0..grid.len() {
        for c in 0..grid[r].len() {
            let above = if r > 0 { grid[r - 1][c] } else { 1 };
            let left = if c > 0 { grid[r][c - 1] } else { 0 };
            grid[r][c] = above + left;
        }
    }
    println!("{:?}", grid);
}
