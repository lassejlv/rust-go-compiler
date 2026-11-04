// Cellular automata on a generic toroidal grid: Conway's Life, a 3-state
// Brian's Brain, and a 1D Wolfram rule, sharing a Rule trait with
// associated types/consts; cycle detection via hashing; pattern parsing.
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

trait Rule {
    type Cell: Copy + Eq + Hash + Default + fmt::Debug;
    const NAME: &'static str;
    fn step(cell: Self::Cell, neighbors: &[Self::Cell; 8]) -> Self::Cell;
    fn glyph(cell: Self::Cell) -> char;
    fn parse(c: char) -> Option<Self::Cell>;
    fn alive(cell: Self::Cell) -> bool { cell != Self::Cell::default() }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Grid<R: Rule, const W: usize, const H: usize> {
    cells: Vec<R::Cell>,
    gen: u64,
    _rule: PhantomData<R>,
}

impl<R: Rule, const W: usize, const H: usize> Grid<R, W, H> {
    fn new() -> Self { Grid { cells: vec![R::Cell::default(); W * H], gen: 0, _rule: PhantomData } }

    fn get(&self, x: isize, y: isize) -> R::Cell {
        let xx = x.rem_euclid(W as isize) as usize;
        let yy = y.rem_euclid(H as isize) as usize;
        self.cells[yy * W + xx]
    }

    fn set(&mut self, x: usize, y: usize, c: R::Cell) { self.cells[(y % H) * W + (x % W)] = c; }

    fn place(&mut self, x: usize, y: usize, pattern: &str) -> Result<usize, String> {
        let mut n = 0;
        for (dy, line) in pattern.lines().enumerate() {
            for (dx, ch) in line.chars().enumerate() {
                let c = R::parse(ch).ok_or_else(|| format!("bad char {:?} at {},{}", ch, dx, dy))?;
                if R::alive(c) {
                    n += 1;
                }
                self.set(x + dx, y + dy, c);
            }
        }
        Ok(n)
    }

    fn step(&self) -> Self {
        let mut next = Vec::with_capacity(W * H);
        for y in 0..H as isize {
            for x in 0..W as isize {
                let nb = [
                    self.get(x - 1, y - 1), self.get(x, y - 1), self.get(x + 1, y - 1),
                    self.get(x - 1, y), self.get(x + 1, y),
                    self.get(x - 1, y + 1), self.get(x, y + 1), self.get(x + 1, y + 1),
                ];
                next.push(R::step(self.get(x, y), &nb));
            }
        }
        Grid { cells: next, gen: self.gen + 1, _rule: PhantomData }
    }

    fn population(&self) -> usize { self.cells.iter().filter(|&&c| R::alive(c)).count() }

    fn fingerprint(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.cells.hash(&mut h);
        h.finish()
    }

    fn bbox(&self) -> Option<(usize, usize, usize, usize)> {
        let mut it = self.cells.iter().enumerate().filter(|(_, &c)| R::alive(c)).map(|(i, _)| (i % W, i / W));
        let first = it.next()?;
        Some(it.fold((first.0, first.1, first.0, first.1), |(a, b, c, d), (x, y)| (a.min(x), b.min(y), c.max(x), d.max(y))))
    }
}

impl<R: Rule, const W: usize, const H: usize> fmt::Display for Grid<R, W, H> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} gen {} pop {} ({}x{})", R::NAME, self.gen, self.population(), W, H)?;
        for row in self.cells.chunks(W) {
            let s: String = row.iter().map(|&c| R::glyph(c)).collect();
            writeln!(f, "|{}|", s)?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Life;
impl Rule for Life {
    type Cell = bool;
    const NAME: &'static str = "Life";
    fn step(cell: bool, nb: &[bool; 8]) -> bool {
        let n = nb.iter().filter(|&&b| b).count();
        matches!((cell, n), (true, 2) | (_, 3))
    }
    fn glyph(c: bool) -> char { if c { '#' } else { '.' } }
    fn parse(c: char) -> Option<bool> { match c { '#' | 'O' | '*' => Some(true), '.' | ' ' => Some(false), _ => None } }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
enum Brain { #[default] Off, On, Dying }

#[derive(Clone, PartialEq, Eq, Hash)]
struct BriansBrain;
impl Rule for BriansBrain {
    type Cell = Brain;
    const NAME: &'static str = "Brian's Brain";
    fn step(cell: Brain, nb: &[Brain; 8]) -> Brain {
        match cell {
            Brain::On => Brain::Dying,
            Brain::Dying => Brain::Off,
            Brain::Off if nb.iter().filter(|&&c| c == Brain::On).count() == 2 => Brain::On,
            Brain::Off => Brain::Off,
        }
    }
    fn glyph(c: Brain) -> char { match c { Brain::Off => ' ', Brain::On => '@', Brain::Dying => 'o' } }
    fn parse(c: char) -> Option<Brain> { match c { '@' => Some(Brain::On), 'o' => Some(Brain::Dying), '.' | ' ' => Some(Brain::Off), _ => None } }
}

fn run_until_cycle<R: Rule, const W: usize, const H: usize>(mut g: Grid<R, W, H>, max: u64) -> (u64, Option<(u64, u64)>, Vec<usize>) {
    let mut seen: HashMap<u64, u64> = HashMap::new();
    let mut pops = Vec::new();
    while g.gen < max {
        let fp = g.fingerprint();
        if let Some(&first) = seen.get(&fp) {
            return (g.gen, Some((first, g.gen - first)), pops);
        }
        seen.insert(fp, g.gen);
        pops.push(g.population());
        g = g.step();
    }
    (g.gen, None, pops)
}

fn wolfram(rule: u8, width: usize, steps: usize) -> Vec<String> {
    let mut row = vec![false; width];
    row[width / 2] = true;
    let mut out = Vec::new();
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
    let mut g: Grid<Life, 16, 10> = Grid::new();
    println!("placed {:?}", g.place(1, 1, ".#.\n..#\n###"));
    println!("placed {:?}", g.place(10, 5, "###"));
    println!("placed {:?}", g.place(10, 0, "##\n##"));
    println!("bad: {:?}", g.place(0, 0, "#x"));
    print!("{}", g);
    for _ in 0..4 {
        g = g.step();
    }
    print!("{}", g);
    println!("bbox {:?}", g.bbox());
    let snapshot = g.clone();
    for _ in 0..64 {
        g = g.step();
    }
    print!("{}", g);
    println!("same cells as 64 gens ago: {}", g.cells == snapshot.cells);

    let mut rp: Grid<Life, 24, 24> = Grid::new();
    rp.place(10, 10, ".##\n##.\n.#.").unwrap();
    let (end, cycle, pops) = run_until_cycle(rp, 2000);
    println!("r-pentomino on 24x24 torus: stopped at gen {}, cycle {:?}", end, cycle);
    let max = pops.iter().enumerate().max_by_key(|&(i, p)| (*p, std::cmp::Reverse(i))).unwrap();
    println!("peak pop {} at gen {}, final pops {:?}", max.1, max.0, &pops[pops.len().saturating_sub(5)..]);

    let mut blink: Grid<Life, 5, 5> = Grid::new();
    blink.place(1, 2, "###").unwrap();
    println!("blinker: {:?}", run_until_cycle(blink, 10));

    let mut b: Grid<BriansBrain, 20, 8> = Grid::new();
    b.place(8, 3, "@@\n@@").unwrap();
    b.place(2, 2, "@o").unwrap();
    for i in 0..4 {
        if i % 2 == 0 {
            print!("{}", b);
        }
        b = b.step();
    }
    let (end, cycle, pops) = run_until_cycle(b, 300);
    println!("brain: end {} cycle {:?} pops head {:?}", end, cycle, &pops[..pops.len().min(12)]);

    for rule in [30u8, 90, 110] {
        println!("rule {}:", rule);
        for l in wolfram(rule, 31, 12) {
            println!("  {}", l);
        }
    }
    println!("{} {} {}", <Life as Rule>::NAME, BriansBrain::NAME, std::mem::size_of::<Grid<Life, 4, 4>>() == std::mem::size_of::<Grid<BriansBrain, 4, 4>>());
}
