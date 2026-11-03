// Conway's game of life on a torus, glider tracking, plus 1D elementary cellular automaton (rule 30/110)
use std::collections::HashSet;
use std::fmt;

#[derive(Clone, PartialEq, Eq)]
struct Life {
    w: usize,
    h: usize,
    cells: Vec<bool>,
}

impl Life {
    fn from_str(s: &str) -> Self {
        let rows: Vec<&str> = s.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let h = rows.len();
        let w = rows[0].len();
        let cells = rows.iter().flat_map(|r| r.chars().map(|c| c == '#')).collect();
        Life { w, h, cells }
    }
    fn get(&self, x: isize, y: isize) -> bool {
        let x = x.rem_euclid(self.w as isize) as usize;
        let y = y.rem_euclid(self.h as isize) as usize;
        self.cells[y * self.w + x]
    }
    fn neighbors(&self, x: usize, y: usize) -> usize {
        let mut n = 0;
        for dy in -1..=1isize {
            for dx in -1..=1isize {
                if (dx, dy) != (0, 0) && self.get(x as isize + dx, y as isize + dy) {
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
                let alive = self.cells[y * self.w + x];
                next.cells[y * self.w + x] = matches!((alive, self.neighbors(x, y)), (true, 2) | (_, 3));
            }
        }
        next
    }
    fn population(&self) -> usize {
        self.cells.iter().filter(|&&c| c).count()
    }
    fn bbox(&self) -> Option<(usize, usize, usize, usize)> {
        let live: Vec<(usize, usize)> = (0..self.h).flat_map(|y| (0..self.w).map(move |x| (x, y))).filter(|&(x, y)| self.cells[y * self.w + x]).collect();
        if live.is_empty() {
            return None;
        }
        let minx = live.iter().map(|p| p.0).min()?;
        let maxx = live.iter().map(|p| p.0).max()?;
        let miny = live.iter().map(|p| p.1).min()?;
        let maxy = live.iter().map(|p| p.1).max()?;
        Some((minx, miny, maxx, maxy))
    }
}

impl fmt::Display for Life {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in self.cells.chunks(self.w) {
            let s: String = row.iter().map(|&c| if c { '#' } else { '.' }).collect();
            writeln!(f, "{}", s)?;
        }
        Ok(())
    }
}

fn elementary(rule: u8, width: usize, steps: usize) -> Vec<String> {
    let mut row = vec![false; width];
    row[width / 2] = true;
    let mut out = vec![];
    for _ in 0..steps {
        out.push(row.iter().map(|&b| if b { '█' } else { ' ' }).collect::<String>().trim_end().to_string());
        row = (0..width)
            .map(|i| {
                let l = row[(i + width - 1) % width] as u8;
                let c = row[i] as u8;
                let r = row[(i + 1) % width] as u8;
                rule >> (l << 2 | c << 1 | r) & 1 == 1
            })
            .collect();
    }
    out
}

fn main() {
    let mut g = Life::from_str(
        "
        .#......
        ..#.....
        ###.....
        ........
        ........
        ........
        ",
    );
    print!("{}", g);
    for gen in 1..=4 {
        g = g.step();
        println!("gen {} pop {} bbox {:?}", gen, g.population(), g.bbox());
    }
    print!("{}", g);
    let mut seen = HashSet::new();
    let mut cur = g.clone();
    let mut period = 0;
    while seen.insert(cur.cells.clone()) {
        cur = cur.step();
        period += 1;
    }
    println!("state repeats after {} steps", period);
    let blinker = Life::from_str(".....\n..#..\n..#..\n..#..\n.....");
    let b1 = blinker.step();
    print!("{}", b1);
    println!("period2 {}", b1.step() == blinker);
    let block = Life::from_str("....\n.##.\n.##.\n....");
    println!("still {}", block.step() == block);
    for line in elementary(30, 31, 12) {
        println!("|{}", line);
    }
    let r110 = elementary(110, 24, 8);
    println!("{}", r110.iter().map(|l| l.chars().filter(|&c| c == '█').count().to_string()).collect::<Vec<_>>().join(","));
}
