use std::collections::HashSet;
use std::fmt;

#[derive(Clone, PartialEq)]
struct Life {
    w: usize,
    h: usize,
    cells: Vec<bool>,
    wrap: bool,
}

impl Life {
    fn from_pattern(w: usize, h: usize, wrap: bool, pattern: &[&str], ox: usize, oy: usize) -> Life {
        let mut l = Life { w, h, cells: vec![false; w * h], wrap };
        for (dy, row) in pattern.iter().enumerate() {
            for (dx, ch) in row.chars().enumerate() {
                if ch == '#' {
                    l.cells[(oy + dy) * w + ox + dx] = true;
                }
            }
        }
        l
    }
    fn alive(&self, x: i64, y: i64) -> bool {
        let (w, h) = (self.w as i64, self.h as i64);
        let (x, y) = if self.wrap {
            (x.rem_euclid(w), y.rem_euclid(h))
        } else if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        } else {
            (x, y)
        };
        self.cells[(y * w + x) as usize]
    }
    fn neighbors(&self, x: usize, y: usize) -> usize {
        let mut n = 0;
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                if (dx, dy) != (0, 0) && self.alive(x as i64 + dx, y as i64 + dy) {
                    n += 1;
                }
            }
        }
        n
    }
    fn step(&self) -> Life {
        let mut next = self.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                let n = self.neighbors(x, y);
                let a = self.cells[y * self.w + x];
                next.cells[y * self.w + x] = matches!((a, n), (true, 2) | (true, 3) | (false, 3));
            }
        }
        next
    }
    fn population(&self) -> usize {
        self.cells.iter().filter(|&&c| c).count()
    }
    fn bbox(&self) -> Option<(usize, usize, usize, usize)> {
        let pts: Vec<(usize, usize)> = (0..self.h)
            .flat_map(|y| (0..self.w).map(move |x| (x, y)))
            .filter(|&(x, y)| self.cells[y * self.w + x])
            .collect();
        if pts.is_empty() {
            return None;
        }
        let minx = pts.iter().map(|p| p.0).min().unwrap();
        let maxx = pts.iter().map(|p| p.0).max().unwrap();
        let miny = pts.iter().map(|p| p.1).min().unwrap();
        let maxy = pts.iter().map(|p| p.1).max().unwrap();
        Some((minx, miny, maxx, maxy))
    }
}

impl fmt::Display for Life {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..self.h {
            let row: String = (0..self.w).map(|x| if self.cells[y * self.w + x] { '#' } else { '.' }).collect();
            writeln!(f, "{}", row)?;
        }
        Ok(())
    }
}

fn find_period(start: &Life, max: usize) -> Option<(usize, usize)> {
    let mut seen: Vec<Life> = vec![start.clone()];
    let mut cur = start.clone();
    for i in 1..=max {
        cur = cur.step();
        if let Some(j) = seen.iter().position(|s| *s == cur) {
            return Some((j, i - j));
        }
        seen.push(cur.clone());
    }
    None
}

fn main() {
    let blinker = Life::from_pattern(5, 5, false, &["###"], 1, 2);
    print!("{}", blinker);
    println!("--");
    print!("{}", blinker.step());
    println!("blinker period {:?}", find_period(&blinker, 10));

    let mut glider = Life::from_pattern(8, 8, true, &[".#.", "..#", "###"], 0, 0);
    for gen in 0..=4 {
        println!("gen {} pop {} bbox {:?}", gen, glider.population(), glider.bbox());
        if gen < 4 {
            glider = glider.step();
        }
    }
    print!("{}", glider);
    println!("glider (torus 8x8) cycle {:?}", find_period(&Life::from_pattern(8, 8, true, &[".#.", "..#", "###"], 0, 0), 40));

    let block = Life::from_pattern(4, 4, false, &["##", "##"], 1, 1);
    println!("block still life: {}", block.step() == block);

    let mut rpent = Life::from_pattern(40, 40, false, &[".##", "##.", ".#."], 18, 18);
    let mut pops = Vec::new();
    for _ in 0..30 {
        rpent = rpent.step();
        pops.push(rpent.population());
    }
    println!("r-pentomino pops {:?}", pops);
    let alive: HashSet<(usize, usize)> = (0..40).flat_map(|y| (0..40).map(move |x| (x, y))).filter(|&(x, y)| rpent.cells[y * 40 + x]).collect();
    println!("alive set size {} bbox {:?}", alive.len(), rpent.bbox());
    let empty = Life::from_pattern(3, 3, false, &["#"], 1, 1);
    println!("lonely cell dies: {}, bbox {:?}", empty.step().population() == 0, empty.step().bbox());
}
