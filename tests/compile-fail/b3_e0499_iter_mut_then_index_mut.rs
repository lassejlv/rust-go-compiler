//~ ERROR E0499
fn main() {
    let mut grid = vec![vec![0u8; 3]; 3];
    for row in grid.iter_mut() {
        grid[0][0] = 1;
        row[1] = 2;
    }
}
