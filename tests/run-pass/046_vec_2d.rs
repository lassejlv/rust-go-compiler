fn print_grid(g: &Vec<Vec<char>>) {
    for row in g {
        let line: String = row.iter().collect();
        println!("{}", line);
    }
}

fn transpose(m: &Vec<Vec<i32>>) -> Vec<Vec<i32>> {
    let rows = m.len();
    let cols = m[0].len();
    let mut t = vec![vec![0; rows]; cols];
    for i in 0..rows {
        for j in 0..cols {
            t[j][i] = m[i][j];
        }
    }
    t
}

fn life_step(g: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let h = g.len() as i32;
    let w = g[0].len() as i32;
    let mut next = vec![vec![false; w as usize]; h as usize];
    for y in 0..h {
        for x in 0..w {
            let mut n = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let ny = y + dy;
                    let nx = x + dx;
                    if ny >= 0 && ny < h && nx >= 0 && nx < w && g[ny as usize][nx as usize] {
                        n += 1;
                    }
                }
            }
            let alive = g[y as usize][x as usize];
            next[y as usize][x as usize] = matches!((alive, n), (true, 2) | (true, 3) | (false, 3));
        }
    }
    next
}

fn main() {
    let mut grid = vec![vec!['.'; 5]; 3];
    grid[1][2] = '#';
    grid[0][0] = '@';
    grid[2][4] = '$';
    print_grid(&grid);
    let m = vec![vec![1, 2, 3], vec![4, 5, 6]];
    println!("m = {:?}", m);
    println!("t = {:?}", transpose(&m));
    let mut tri: Vec<Vec<u64>> = Vec::new();
    for i in 0..8 {
        let mut row = vec![1u64; i + 1];
        for j in 1..i {
            row[j] = tri[i - 1][j - 1] + tri[i - 1][j];
        }
        tri.push(row);
    }
    for row in &tri {
        let strs: Vec<String> = row.iter().map(|x| x.to_string()).collect();
        println!("{}", strs.join(" "));
    }
    let mut g = vec![vec![false; 6]; 6];
    g[1][2] = true;
    g[2][3] = true;
    g[3][1] = true;
    g[3][2] = true;
    g[3][3] = true;
    for gen in 0..3 {
        println!("gen {}:", gen);
        for row in &g {
            let s: String = row.iter().map(|&c| if c { '#' } else { '.' }).collect();
            println!("{}", s);
        }
        g = life_step(&g);
    }
    let row_sums: Vec<i32> = m.iter().map(|r| r.iter().sum()).collect();
    let col_sums: Vec<i32> = (0..3).map(|j| m.iter().map(|r| r[j]).sum()).collect();
    println!("row sums {:?} col sums {:?}", row_sums, col_sums);
}
