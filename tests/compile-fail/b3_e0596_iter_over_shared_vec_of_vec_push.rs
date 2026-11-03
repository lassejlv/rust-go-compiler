//~ ERROR E0596
fn main() {
    let mut grid: Vec<Vec<i32>> = vec![vec![]];
    for row in grid.iter() {
        row.push(1);
    }
    grid.clear();
}
