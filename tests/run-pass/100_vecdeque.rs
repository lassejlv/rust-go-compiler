use std::collections::VecDeque;

fn bfs(grid: &[&str], start: (usize, usize), goal: (usize, usize)) -> Option<usize> {
    let h = grid.len();
    let w = grid[0].len();
    let cells: Vec<Vec<char>> = grid.iter().map(|r| r.chars().collect()).collect();
    let mut dist = vec![vec![usize::MAX; w]; h];
    let mut q = VecDeque::new();
    dist[start.1][start.0] = 0;
    q.push_back(start);
    while let Some((x, y)) = q.pop_front() {
        if (x, y) == goal {
            return Some(dist[y][x]);
        }
        let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        for (dx, dy) in dirs.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if cells[ny][nx] == '#' || dist[ny][nx] != usize::MAX {
                continue;
            }
            dist[ny][nx] = dist[y][x] + 1;
            q.push_back((nx, ny));
        }
    }
    None
}

fn main() {
    let mut d: VecDeque<i32> = VecDeque::new();
    d.push_back(1);
    d.push_back(2);
    d.push_front(0);
    d.push_front(-1);
    println!("deque = {:?} len {}", d, d.len());
    println!("front = {:?}, back = {:?}", d.front(), d.back());
    println!("pop_front = {:?}, pop_back = {:?}", d.pop_front(), d.pop_back());
    println!("deque = {:?}", d);
    d.extend([5, 6, 7]);
    d.rotate_left(2);
    println!("rotate_left 2 = {:?}", d);
    d.rotate_right(1);
    println!("rotate_right 1 = {:?}", d);
    println!("d[1] = {}, get(10) = {:?}", d[1], d.get(10));
    println!("contains 6: {}", d.contains(&6));
    let v: Vec<i32> = d.iter().map(|x| x * 10).collect();
    println!("mapped = {:?}", v);
    let mut window: VecDeque<i32> = VecDeque::with_capacity(3);
    let mut sums = Vec::new();
    for x in [4, 8, 15, 16, 23, 42] {
        if window.len() == 3 {
            window.pop_front();
        }
        window.push_back(x);
        sums.push(window.iter().sum::<i32>());
    }
    println!("sliding sums = {:?}", sums);
    let mut people: VecDeque<u32> = (1..=7).collect();
    let mut order = Vec::new();
    while !people.is_empty() {
        for _ in 0..2 {
            let p = people.pop_front().unwrap();
            people.push_back(p);
        }
        order.push(people.pop_front().unwrap());
    }
    println!("josephus order = {:?}", order);
    let maze = ["S..#....", ".#.#.##.", ".#...#..", ".####.#.", "......#G"];
    println!("bfs distance = {:?}", bfs(&maze, (0, 0), (7, 4)));
    let blocked = ["S#", "#G"];
    println!("bfs blocked = {:?}", bfs(&blocked, (0, 0), (1, 1)));
    let (a, b) = d.as_slices();
    println!("total via slices = {}", a.iter().chain(b.iter()).sum::<i32>());
    let v2: Vec<i32> = d.into_iter().collect();
    println!("into vec = {:?}", v2);
}
