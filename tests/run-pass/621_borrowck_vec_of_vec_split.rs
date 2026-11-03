// Vec<Vec<T>>: mutating two different rows via split_at_mut and swap.
fn add_row(grid: &mut Vec<Vec<i32>>, dst: usize, src: usize) {
    if dst < src {
        let (a, b) = grid.split_at_mut(src);
        for (x, y) in a[dst].iter_mut().zip(b[0].iter()) {
            *x += *y;
        }
    } else {
        let (a, b) = grid.split_at_mut(dst);
        for (x, y) in b[0].iter_mut().zip(a[src].iter()) {
            *x += *y;
        }
    }
}

fn main() {
    let mut g = vec![vec![1, 2, 3], vec![10, 20, 30], vec![100, 200, 300]];
    add_row(&mut g, 0, 2);
    add_row(&mut g, 2, 1);
    g.swap(0, 1);
    let n = g[0].len();
    g[1][n - 1] = g[0][0] + g[2][0];
    let row = g[2].clone();
    g[0].extend(row);
    let total = g[1].iter().sum::<i32>();
    g[2].push(total);
    println!("{:?}", g);
}
