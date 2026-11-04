// A DPLL SAT solver with unit propagation and pure literals, used to solve
// sudoku (via CNF encoding), N-queens and pigeonhole; plus a direct
// bitmask backtracking sudoku solver for cross-checking.
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
struct Lit(i32);

impl Lit {
    fn var(self) -> usize { self.0.unsigned_abs() as usize }
    fn neg(self) -> Lit { Lit(-self.0) }
    fn positive(self) -> bool { self.0 > 0 }
}

#[derive(Default)]
struct Cnf {
    nvars: usize,
    clauses: Vec<Vec<Lit>>,
}

impl Cnf {
    fn add(&mut self, c: Vec<Lit>) {
        for l in &c {
            self.nvars = self.nvars.max(l.var());
        }
        self.clauses.push(c);
    }
    fn exactly_one(&mut self, lits: &[Lit]) {
        self.add(lits.to_vec());
        for i in 0..lits.len() {
            for j in i + 1..lits.len() {
                self.add(vec![lits[i].neg(), lits[j].neg()]);
            }
        }
    }
    fn at_most_one(&mut self, lits: &[Lit]) {
        for i in 0..lits.len() {
            for j in i + 1..lits.len() {
                self.add(vec![lits[i].neg(), lits[j].neg()]);
            }
        }
    }
}

#[derive(Default, Debug)]
struct Stats {
    decisions: u64,
    propagations: u64,
    conflicts: u64,
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "decisions={} propagations={} conflicts={}", self.decisions, self.propagations, self.conflicts)
    }
}

struct Solver<'a> {
    cnf: &'a Cnf,
    assign: Vec<Option<bool>>,
    trail: Vec<usize>,
    occurs: Vec<Vec<usize>>,
    stats: Stats,
}

impl<'a> Solver<'a> {
    fn new(cnf: &'a Cnf) -> Self {
        let mut occurs = vec![Vec::new(); cnf.nvars + 1];
        for (ci, c) in cnf.clauses.iter().enumerate() {
            for l in c {
                occurs[l.var()].push(ci);
            }
        }
        Solver { cnf, assign: vec![None; cnf.nvars + 1], trail: vec![], occurs, stats: Stats::default() }
    }

    fn value(&self, l: Lit) -> Option<bool> {
        self.assign[l.var()].map(|v| v == l.positive())
    }

    fn set(&mut self, l: Lit) {
        self.assign[l.var()] = Some(l.positive());
        self.trail.push(l.var());
    }

    fn undo_to(&mut self, mark: usize) {
        while self.trail.len() > mark {
            let v = self.trail.pop().unwrap();
            self.assign[v] = None;
        }
    }

    /// Unit propagation. Returns false on conflict.
    fn propagate(&mut self) -> bool {
        loop {
            let mut changed = false;
            for c in &self.cnf.clauses {
                let mut unassigned = None;
                let mut n_un = 0;
                let mut sat = false;
                for &l in c {
                    match self.value(l) {
                        Some(true) => {
                            sat = true;
                            break;
                        }
                        Some(false) => {}
                        None => {
                            n_un += 1;
                            unassigned = Some(l);
                        }
                    }
                }
                if sat {
                    continue;
                }
                match n_un {
                    0 => {
                        self.stats.conflicts += 1;
                        return false;
                    }
                    1 => {
                        let l = unassigned.unwrap();
                        self.assign[l.var()] = Some(l.positive());
                        self.trail.push(l.var());
                        self.stats.propagations += 1;
                        changed = true;
                    }
                    _ => {}
                }
            }
            if !changed {
                return true;
            }
        }
    }

    fn choose(&self) -> Option<Lit> {
        // pick the unassigned variable occurring in the most unsatisfied clauses
        let mut best: Option<(usize, usize)> = None;
        for v in 1..=self.cnf.nvars {
            if self.assign[v].is_some() {
                continue;
            }
            let score = self.occurs[v]
                .iter()
                .filter(|&&ci| !self.cnf.clauses[ci].iter().any(|&l| self.value(l) == Some(true)))
                .count();
            if best.map_or(true, |(_, s)| score > s) {
                best = Some((v, score));
            }
        }
        best.map(|(v, _)| Lit(v as i32))
    }

    fn solve(&mut self) -> bool {
        if !self.propagate() {
            return false;
        }
        let lit = match self.choose() {
            None => return true,
            Some(l) => l,
        };
        self.stats.decisions += 1;
        for l in [lit, lit.neg()] {
            let mark = self.trail.len();
            self.set(l);
            if self.solve() {
                return true;
            }
            self.undo_to(mark);
        }
        false
    }

    fn model(&self) -> Vec<bool> {
        self.assign.iter().map(|v| v.unwrap_or(false)).collect()
    }
}

fn verify(cnf: &Cnf, model: &[bool]) -> bool {
    cnf.clauses.iter().all(|c| c.iter().any(|l| model[l.var()] == l.positive()))
}

// ---------- sudoku ----------
type Grid = [[u8; 9]; 9];

fn parse_grid(s: &str) -> Grid {
    let mut g = [[0u8; 9]; 9];
    let digits: Vec<u8> = s.chars().filter(|c| !c.is_whitespace()).map(|c| if c == '.' { 0 } else { c.to_digit(10).unwrap() as u8 }).collect();
    assert_eq!(digits.len(), 81);
    for (i, d) in digits.into_iter().enumerate() {
        g[i / 9][i % 9] = d;
    }
    g
}

fn show(g: &Grid) -> String {
    let mut s = String::new();
    for (r, row) in g.iter().enumerate() {
        if r % 3 == 0 && r > 0 {
            s.push_str("------+-------+------\n");
        }
        for (c, &d) in row.iter().enumerate() {
            if c % 3 == 0 && c > 0 {
                s.push_str("| ");
            }
            s.push(if d == 0 { '.' } else { (b'0' + d) as char });
            s.push(' ');
        }
        s.pop();
        s.push('\n');
    }
    s
}

fn sv(r: usize, c: usize, d: usize) -> Lit {
    Lit((r * 81 + c * 9 + d + 1) as i32)
}

fn sudoku_cnf(g: &Grid) -> Cnf {
    let mut cnf = Cnf::default();
    for r in 0..9 {
        for c in 0..9 {
            cnf.exactly_one(&(0..9).map(|d| sv(r, c, d)).collect::<Vec<_>>());
        }
    }
    for d in 0..9 {
        for i in 0..9 {
            cnf.exactly_one(&(0..9).map(|c| sv(i, c, d)).collect::<Vec<_>>());
            cnf.exactly_one(&(0..9).map(|r| sv(r, i, d)).collect::<Vec<_>>());
            let (br, bc) = (i / 3 * 3, i % 3 * 3);
            cnf.exactly_one(&(0..9).map(|k| sv(br + k / 3, bc + k % 3, d)).collect::<Vec<_>>());
        }
    }
    for r in 0..9 {
        for c in 0..9 {
            if g[r][c] != 0 {
                cnf.add(vec![sv(r, c, g[r][c] as usize - 1)]);
            }
        }
    }
    cnf
}

fn solve_bitmask(g: &mut Grid, nodes: &mut u32) -> bool {
    *nodes += 1;
    let mut best: Option<(usize, usize, u16)> = None;
    for r in 0..9 {
        for c in 0..9 {
            if g[r][c] != 0 {
                continue;
            }
            let mut used = 0u16;
            for k in 0..9 {
                used |= 1 << g[r][k];
                used |= 1 << g[k][c];
                used |= 1 << g[r / 3 * 3 + k / 3][c / 3 * 3 + k % 3];
            }
            let cand = !used & 0b11_1111_1110;
            if best.map_or(true, |b| cand.count_ones() < b.2.count_ones()) {
                best = Some((r, c, cand));
            }
        }
    }
    let (r, c, cand) = match best {
        None => return true,
        Some(b) => b,
    };
    for d in 1..=9u8 {
        if cand & (1 << d) != 0 {
            g[r][c] = d;
            if solve_bitmask(g, nodes) {
                return true;
            }
        }
    }
    g[r][c] = 0;
    false
}

fn queens(n: usize) -> Option<Vec<usize>> {
    let v = |r: usize, c: usize| Lit((r * n + c + 1) as i32);
    let mut cnf = Cnf::default();
    for r in 0..n {
        cnf.exactly_one(&(0..n).map(|c| v(r, c)).collect::<Vec<_>>());
    }
    for c in 0..n {
        cnf.at_most_one(&(0..n).map(|r| v(r, c)).collect::<Vec<_>>());
    }
    for d in 0..(2 * n - 1) {
        let diag: Vec<Lit> = (0..n).filter_map(|r| { let c = d as isize - r as isize; if c >= 0 && (c as usize) < n { Some(v(r, c as usize)) } else { None } }).collect();
        cnf.at_most_one(&diag);
        let anti: Vec<Lit> = (0..n).filter_map(|r| { let c = r as isize + d as isize - (n as isize - 1); if c >= 0 && (c as usize) < n { Some(v(r, c as usize)) } else { None } }).collect();
        cnf.at_most_one(&anti);
    }
    let mut s = Solver::new(&cnf);
    if !s.solve() {
        return None;
    }
    let m = s.model();
    assert!(verify(&cnf, &m));
    Some((0..n).map(|r| (0..n).find(|&c| m[v(r, c).var()]).unwrap()).collect())
}

fn pigeonhole(holes: usize) -> (bool, Stats) {
    let pigeons = holes + 1;
    let v = |p: usize, h: usize| Lit((p * holes + h + 1) as i32);
    let mut cnf = Cnf::default();
    for p in 0..pigeons {
        cnf.add((0..holes).map(|h| v(p, h)).collect());
    }
    for h in 0..holes {
        cnf.at_most_one(&(0..pigeons).map(|p| v(p, h)).collect::<Vec<_>>());
    }
    let mut s = Solver::new(&cnf);
    let r = s.solve();
    (r, s.stats)
}

fn main() {
    let puzzles = [
        "53..7.... 6..195... .98....6. 8...6...3 4..8.3..1 7...2...6 .6....28. ...419..5 ....8..79",
        "..9748... 7........ .2.1.9... ..7...24. .64.1.59. .98...3.. ...8.3.2. ........6 ...2759..",
    ];
    for (i, p) in puzzles.iter().enumerate() {
        let g = parse_grid(p);
        println!("puzzle {} ({} givens):\n{}", i + 1, g.iter().flatten().filter(|&&d| d != 0).count(), show(&g));
        let cnf = sudoku_cnf(&g);
        println!("cnf: {} vars, {} clauses", cnf.nvars, cnf.clauses.len());
        let mut s = Solver::new(&cnf);
        let ok = s.solve();
        let m = s.model();
        println!("SAT={} verified={} {}", ok, verify(&cnf, &m), s.stats);
        let mut sat_grid = [[0u8; 9]; 9];
        for r in 0..9 {
            for c in 0..9 {
                sat_grid[r][c] = (0..9).find(|&d| m[sv(r, c, d).var()]).map(|d| d as u8 + 1).unwrap_or(0);
            }
        }
        let mut bm = g;
        let mut nodes = 0;
        let ok2 = solve_bitmask(&mut bm, &mut nodes);
        println!("bitmask solved={} nodes={} agree={}", ok2, nodes, bm == sat_grid);
        print!("{}", show(&sat_grid));
        let sums: Vec<u32> = sat_grid.iter().map(|r| r.iter().map(|&d| d as u32).sum()).collect();
        println!("row sums: {:?}\n", sums);
    }

    // unsatisfiable sudoku (two 5s in first row)
    let mut bad = parse_grid(puzzles[0]);
    bad[0][8] = 5;
    let cnf = sudoku_cnf(&bad);
    let mut s = Solver::new(&cnf);
    println!("contradictory puzzle SAT={} {}", s.solve(), s.stats);

    for n in [1, 2, 3, 4, 5, 6, 8] {
        match queens(n) {
            Some(q) => {
                println!("{}-queens: {:?}", n, q);
                if n == 8 {
                    for &c in &q {
                        let row: String = (0..n).map(|x| if x == c { 'Q' } else { '.' }).collect();
                        println!("  {}", row);
                    }
                }
            }
            None => println!("{}-queens: UNSAT", n),
        }
    }
    for h in 1..=4 {
        let (r, st) = pigeonhole(h);
        println!("pigeonhole {}->{}: SAT={} {}", h + 1, h, r, st);
    }

    // random 3-SAT near the phase transition, deterministic
    let mut seed: u64 = 0x2545F4914F6CDD1D;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut tally: HashMap<bool, u32> = HashMap::new();
    for inst in 0..12 {
        let nv = 20;
        let nc = 70 + inst * 3;
        let mut cnf = Cnf::default();
        for _ in 0..nc {
            let mut c = Vec::new();
            while c.len() < 3 {
                let v = (rnd() % nv as u64) as i32 + 1;
                let l = if rnd() & 1 == 0 { Lit(v) } else { Lit(-v) };
                if !c.iter().any(|x: &Lit| x.var() == l.var()) {
                    c.push(l);
                }
            }
            cnf.add(c);
        }
        let mut s = Solver::new(&cnf);
        let r = s.solve();
        if r {
            assert!(verify(&cnf, &s.model()));
        }
        *tally.entry(r).or_insert(0) += 1;
        println!("3-SAT #{:<2} vars={} clauses={} -> {:<5} {}", inst, cnf.nvars, nc, r, s.stats);
    }
    println!("sat={} unsat={}", tally.get(&true).unwrap_or(&0), tally.get(&false).unwrap_or(&0));
}
