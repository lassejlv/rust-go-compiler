// Constraint solvers: DPLL SAT with unit propagation and pure literals,
// Sudoku via SAT encoding and via backtracking with bitmasks, N-queens
// counting, plus a generic backtracking framework using ControlFlow.
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::ops::ControlFlow;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Lit(i32);

impl Lit {
    fn var(self) -> usize {
        self.0.unsigned_abs() as usize
    }
    fn neg(self) -> Lit {
        Lit(-self.0)
    }
    fn positive(self) -> bool {
        self.0 > 0
    }
}

impl fmt::Debug for Lit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.positive() {
            write!(f, "x{}", self.var())
        } else {
            write!(f, "¬x{}", self.var())
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Cnf {
    nvars: usize,
    clauses: Vec<Vec<Lit>>,
}

impl Cnf {
    fn add(&mut self, c: impl IntoIterator<Item = i32>) {
        let mut cl: Vec<Lit> = c.into_iter().map(Lit).collect();
        cl.sort();
        cl.dedup();
        if cl.windows(2).any(|w| w[0].var() == w[1].var()) {
            return; // tautology
        }
        for l in &cl {
            self.nvars = self.nvars.max(l.var());
        }
        self.clauses.push(cl);
    }
    fn exactly_one(&mut self, vars: &[i32]) {
        self.add(vars.iter().copied());
        for (i, &a) in vars.iter().enumerate() {
            for &b in &vars[i + 1..] {
                self.add([-a, -b]);
            }
        }
    }
    fn to_dimacs(&self) -> String {
        let mut s = format!("p cnf {} {}\n", self.nvars, self.clauses.len());
        for c in self.clauses.iter().take(3) {
            for l in c {
                s += &format!("{} ", l.0);
            }
            s += "0\n";
        }
        s
    }
}

#[derive(Default, Debug)]
struct SatStats {
    decisions: u64,
    propagations: u64,
    conflicts: u64,
    pure: u64,
}

struct Dpll<'c> {
    cnf: &'c Cnf,
    assign: Vec<Option<bool>>,
    trail: Vec<usize>,
    occurs: Vec<Vec<usize>>,
    stats: SatStats,
}

enum Status {
    Sat,
    Conflict,
    Unit(Lit),
    Open,
}

impl<'c> Dpll<'c> {
    fn new(cnf: &'c Cnf) -> Self {
        let mut occurs = vec![Vec::new(); cnf.nvars + 1];
        for (i, c) in cnf.clauses.iter().enumerate() {
            for l in c {
                occurs[l.var()].push(i);
            }
        }
        Dpll { cnf, assign: vec![None; cnf.nvars + 1], trail: vec![], occurs, stats: SatStats::default() }
    }
    fn value(&self, l: Lit) -> Option<bool> {
        self.assign[l.var()].map(|v| v == l.positive())
    }
    fn set(&mut self, l: Lit) {
        self.assign[l.var()] = Some(l.positive());
        self.trail.push(l.var());
    }
    fn undo_to(&mut self, mark: usize) {
        for v in self.trail.drain(mark..) {
            self.assign[v] = None;
        }
    }
    fn clause_status(&self, c: &[Lit]) -> Status {
        let mut unassigned = None;
        let mut n = 0;
        for &l in c {
            match self.value(l) {
                Some(true) => return Status::Sat,
                Some(false) => {}
                None => {
                    n += 1;
                    unassigned = Some(l);
                }
            }
        }
        match (n, unassigned) {
            (0, _) => Status::Conflict,
            (1, Some(l)) => Status::Unit(l),
            _ => Status::Open,
        }
    }
    fn propagate(&mut self) -> bool {
        let mut queue: VecDeque<usize> = (0..self.cnf.clauses.len()).collect();
        let mut queued = vec![true; self.cnf.clauses.len()];
        while let Some(ci) = queue.pop_front() {
            queued[ci] = false;
            match self.clause_status(&self.cnf.clauses[ci]) {
                Status::Conflict => {
                    self.stats.conflicts += 1;
                    return false;
                }
                Status::Unit(l) => {
                    self.stats.propagations += 1;
                    self.set(l);
                    for &cj in &self.occurs[l.var()] {
                        if !queued[cj] {
                            queued[cj] = true;
                            queue.push_back(cj);
                        }
                    }
                }
                _ => {}
            }
        }
        true
    }
    fn pure_literals(&mut self) {
        let mut pol: BTreeMap<usize, (bool, bool)> = BTreeMap::new();
        for c in &self.cnf.clauses {
            if matches!(self.clause_status(c), Status::Sat) {
                continue;
            }
            for &l in c.iter().filter(|l| self.value(**l).is_none()) {
                let e = pol.entry(l.var()).or_insert((false, false));
                if l.positive() {
                    e.0 = true
                } else {
                    e.1 = true
                }
            }
        }
        for (v, (p, n)) in pol {
            if p != n {
                self.stats.pure += 1;
                self.set(Lit(if p { v as i32 } else { -(v as i32) }));
            }
        }
    }
    fn choose(&self) -> Option<Lit> {
        // most occurrences among unsatisfied clauses
        let mut score: HashMap<i32, usize> = HashMap::new();
        for c in &self.cnf.clauses {
            if let Status::Sat = self.clause_status(c) {
                continue;
            }
            for &l in c {
                if self.value(l).is_none() {
                    *score.entry(l.0).or_insert(0) += 1;
                }
            }
        }
        score.into_iter().max_by_key(|&(l, s)| (s, -l.abs(), l)).map(|(l, _)| Lit(l))
    }
    fn solve(&mut self) -> bool {
        let mark = self.trail.len();
        if !self.propagate() {
            self.undo_to(mark);
            return false;
        }
        self.pure_literals();
        let Some(l) = self.choose() else {
            return true;
        };
        self.stats.decisions += 1;
        for lit in [l, l.neg()] {
            let m2 = self.trail.len();
            self.set(lit);
            if self.solve() {
                return true;
            }
            self.undo_to(m2);
        }
        self.undo_to(mark);
        false
    }
    fn check_model(&self) -> bool {
        self.cnf.clauses.iter().all(|c| c.iter().any(|&l| self.value(l) != Some(false)))
    }
}

// ---------------- sudoku ----------------
type Grid = [[u8; 9]; 9];

fn parse_grid(s: &str) -> Grid {
    let mut g = [[0u8; 9]; 9];
    for (i, ch) in s.chars().filter(|c| !c.is_whitespace()).enumerate() {
        g[i / 9][i % 9] = ch.to_digit(10).unwrap_or(0) as u8;
    }
    g
}

fn show_grid(g: &Grid) -> String {
    g.iter()
        .enumerate()
        .map(|(r, row)| {
            let s: String = row.chunks(3).map(|c| c.iter().map(|d| if *d == 0 { '.' } else { char::from_digit(*d as u32, 10).unwrap() }).collect::<String>()).collect::<Vec<_>>().join("|");
            if r % 3 == 2 && r != 8 {
                format!("{}\n---+---+---", s)
            } else {
                s
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn sudoku_cnf(g: &Grid) -> Cnf {
    let v = |r: usize, c: usize, d: usize| (r * 81 + c * 9 + d + 1) as i32;
    let mut cnf = Cnf::default();
    for r in 0..9 {
        for c in 0..9 {
            cnf.exactly_one(&(0..9).map(|d| v(r, c, d)).collect::<Vec<_>>());
            if g[r][c] != 0 {
                cnf.add([v(r, c, g[r][c] as usize - 1)]);
            }
        }
    }
    for d in 0..9 {
        for i in 0..9 {
            cnf.exactly_one(&(0..9).map(|c| v(i, c, d)).collect::<Vec<_>>());
            cnf.exactly_one(&(0..9).map(|r| v(r, i, d)).collect::<Vec<_>>());
            let (br, bc) = (i / 3 * 3, i % 3 * 3);
            cnf.exactly_one(&(0..9).map(|k| v(br + k / 3, bc + k % 3, d)).collect::<Vec<_>>());
        }
    }
    cnf
}

struct BitSudoku {
    g: Grid,
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
    nodes: u64,
}

impl BitSudoku {
    fn new(g: Grid) -> Option<Self> {
        let mut s = BitSudoku { g, rows: [0; 9], cols: [0; 9], boxes: [0; 9], nodes: 0 };
        for r in 0..9 {
            for c in 0..9 {
                let d = g[r][c];
                if d != 0 {
                    let bit = 1 << d;
                    if (s.rows[r] | s.cols[c] | s.boxes[r / 3 * 3 + c / 3]) & bit != 0 {
                        return None;
                    }
                    s.rows[r] |= bit;
                    s.cols[c] |= bit;
                    s.boxes[r / 3 * 3 + c / 3] |= bit;
                }
            }
        }
        Some(s)
    }
    fn solve(&mut self, count_limit: usize, found: &mut Vec<Grid>) {
        self.nodes += 1;
        // MRV: the empty cell with fewest candidates
        let mut best: Option<(usize, usize, u16)> = None;
        for r in 0..9 {
            for c in 0..9 {
                if self.g[r][c] == 0 {
                    let cand = !(self.rows[r] | self.cols[c] | self.boxes[r / 3 * 3 + c / 3]) & 0b11_1111_1110;
                    if best.map_or(true, |(_, _, b)| cand.count_ones() < b.count_ones()) {
                        best = Some((r, c, cand));
                    }
                }
            }
        }
        let Some((r, c, cand)) = best else {
            found.push(self.g);
            return;
        };
        let mut bits = cand;
        while bits != 0 && found.len() < count_limit {
            let d = bits.trailing_zeros() as u8;
            bits &= bits - 1;
            let bit = 1u16 << d;
            let b = r / 3 * 3 + c / 3;
            self.g[r][c] = d;
            self.rows[r] |= bit;
            self.cols[c] |= bit;
            self.boxes[b] |= bit;
            self.solve(count_limit, found);
            self.rows[r] ^= bit;
            self.cols[c] ^= bit;
            self.boxes[b] ^= bit;
            self.g[r][c] = 0;
        }
    }
}

fn valid_solution(g: &Grid) -> bool {
    let full = |it: &mut dyn Iterator<Item = u8>| it.fold(0u16, |m, d| m | 1 << d) == 0b11_1111_1110;
    (0..9).all(|i| {
        full(&mut g[i].iter().copied()) && full(&mut (0..9).map(|r| g[r][i])) && full(&mut (0..9).map(|k| g[i / 3 * 3 + k / 3][i % 3 * 3 + k % 3]))
    })
}

// ---------------- generic backtracking with ControlFlow ----------------
trait Search {
    type Move: Copy + fmt::Debug;
    fn moves(&self) -> Vec<Self::Move>;
    fn apply(&mut self, m: Self::Move);
    fn undo(&mut self, m: Self::Move);
    fn is_goal(&self) -> bool;

    fn search<B>(&mut self, visit: &mut impl FnMut(&Self) -> ControlFlow<B>) -> ControlFlow<B>
    where
        Self: Sized,
    {
        if self.is_goal() {
            return visit(self);
        }
        for m in self.moves() {
            self.apply(m);
            let r = self.search(visit);
            self.undo(m);
            r?;
        }
        ControlFlow::Continue(())
    }
}

struct Queens {
    n: usize,
    cols: Vec<usize>,
    used_c: u32,
    used_d1: u64,
    used_d2: u64,
}

impl Search for Queens {
    type Move = usize;
    fn moves(&self) -> Vec<usize> {
        let r = self.cols.len();
        (0..self.n).filter(|&c| self.used_c >> c & 1 == 0 && self.used_d1 >> (r + c) & 1 == 0 && self.used_d2 >> (r + self.n - c) & 1 == 0).collect()
    }
    fn apply(&mut self, c: usize) {
        let r = self.cols.len();
        self.used_c |= 1 << c;
        self.used_d1 |= 1 << (r + c);
        self.used_d2 |= 1 << (r + self.n - c);
        self.cols.push(c);
    }
    fn undo(&mut self, c: usize) {
        self.cols.pop();
        let r = self.cols.len();
        self.used_c &= !(1 << c);
        self.used_d1 &= !(1 << (r + c));
        self.used_d2 &= !(1 << (r + self.n - c));
    }
    fn is_goal(&self) -> bool {
        self.cols.len() == self.n
    }
}

struct SubsetSum<'a> {
    items: &'a [u32],
    idx: usize,
    chosen: Vec<u32>,
    sum: u32,
    target: u32,
}

impl<'a> Search for SubsetSum<'a> {
    type Move = Option<u32>;
    fn moves(&self) -> Vec<Option<u32>> {
        if self.idx >= self.items.len() || self.sum > self.target {
            return vec![];
        }
        vec![Some(self.items[self.idx]), None]
    }
    fn apply(&mut self, m: Option<u32>) {
        self.idx += 1;
        if let Some(x) = m {
            self.chosen.push(x);
            self.sum += x;
        }
    }
    fn undo(&mut self, m: Option<u32>) {
        self.idx -= 1;
        if let Some(x) = m {
            self.chosen.pop();
            self.sum -= x;
        }
    }
    fn is_goal(&self) -> bool {
        self.sum == self.target
    }
}

fn main() {
    // small SAT instances
    let mut cnf = Cnf::default();
    for c in [vec![1, 2, -3], vec![-1, 3], vec![-2, 3], vec![-3, 4], vec![-4, -1, 2], vec![1, -1]] {
        cnf.add(c);
    }
    let mut s = Dpll::new(&cnf);
    let sat = s.solve();
    println!("small: sat={} model={:?} ok={}", sat, (1..=cnf.nvars).map(|v| s.assign[v]).collect::<Vec<_>>(), s.check_model());
    print!("{}", cnf.to_dimacs());
    // pigeonhole 5 into 4: unsat
    let mut php = Cnf::default();
    let p = |i: i32, h: i32| i * 4 + h + 1;
    for i in 0..5 {
        php.add((0..4).map(|h| p(i, h)));
    }
    for h in 0..4 {
        for i in 0..5 {
            for j in i + 1..5 {
                php.add([-p(i, h), -p(j, h)]);
            }
        }
    }
    let mut s = Dpll::new(&php);
    println!("pigeonhole(5,4): sat={} {:?}", s.solve(), s.stats);
    // random 3-SAT near the threshold, deterministic
    let mut seed = 99u64;
    let mut rnd = |n: u64| {
        seed = seed.wrapping_mul(0x5851F42D4C957F2D).wrapping_add(11);
        (seed >> 33) % n
    };
    let mut results = vec![];
    for inst in 0..6 {
        let nv = 20;
        let mut f = Cnf::default();
        for _ in 0..(nv as f64 * (3.6 + inst as f64 * 0.2)) as usize {
            let c: Vec<i32> = (0..3).map(|_| (rnd(nv) as i32 + 1) * if rnd(2) == 0 { 1 } else { -1 }).collect();
            f.add(c);
        }
        f.nvars = nv as usize;
        let mut s = Dpll::new(&f);
        let r = s.solve();
        assert!(!r || s.check_model());
        results.push(format!("{}:{}({})", f.clauses.len(), if r { "SAT" } else { "UNSAT" }, s.stats.decisions));
    }
    println!("random 3-SAT: {}", results.join(" "));

    // sudoku
    let puzzle = "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    let g = parse_grid(puzzle);
    let cnf = sudoku_cnf(&g);
    let mut s = Dpll::new(&cnf);
    let ok = s.solve();
    let mut sol = [[0u8; 9]; 9];
    for (i, a) in s.assign.iter().enumerate().skip(1) {
        if *a == Some(true) {
            let k = i - 1;
            sol[k / 81][k / 9 % 9] = (k % 9 + 1) as u8;
        }
    }
    println!("sudoku via SAT: vars={} clauses={} sat={} valid={} decisions={}", cnf.nvars, cnf.clauses.len(), ok, valid_solution(&sol), s.stats.decisions);
    println!("{}", show_grid(&sol));
    let mut bs = BitSudoku::new(g).unwrap();
    let mut found = vec![];
    bs.solve(2, &mut found);
    println!("bitmask solver: {} solution(s), nodes {}, same as SAT: {}", found.len(), bs.nodes, found.first() == Some(&sol));
    let hard = parse_grid("000000010400000000020000000000050407008000300001090000300400200050100000000806000");
    let mut hs = BitSudoku::new(hard).unwrap();
    let mut hf = vec![];
    hs.solve(2, &mut hf);
    println!("hard: {} solution(s), nodes {}, valid {}", hf.len(), hs.nodes, hf.iter().all(valid_solution));
    let mut multi = g;
    multi[0][0] = 0;
    multi[0][1] = 0;
    multi[1][0] = 0;
    multi[4][4] = 0;
    let mut ms = BitSudoku::new(multi).unwrap();
    let mut mf = vec![];
    ms.solve(5, &mut mf);
    println!("relaxed: {} solution(s)", mf.len());
    let mut bad = g;
    bad[0][2] = 5;
    println!("conflicting givens rejected: {}", BitSudoku::new(bad).is_none());

    // n-queens via generic search
    for n in [4usize, 5, 6, 8] {
        let mut q = Queens { n, cols: vec![], used_c: 0, used_d1: 0, used_d2: 0 };
        let mut count = 0;
        let mut first: Option<Vec<usize>> = None;
        let _ = q.search(&mut |s: &Queens| {
            count += 1;
            first.get_or_insert_with(|| s.cols.clone());
            ControlFlow::<()>::Continue(())
        });
        println!("{}-queens: {} solutions, first {:?}", n, count, first);
    }
    let mut q = Queens { n: 10, cols: vec![], used_c: 0, used_d1: 0, used_d2: 0 };
    let r = q.search(&mut |s: &Queens| if s.cols[0] >= 2 { ControlFlow::Break(s.cols.clone()) } else { ControlFlow::Continue(()) });
    println!("10-queens first with col0>=2: {:?}", r);

    let items = [3u32, 34, 4, 12, 5, 2, 27, 9];
    let mut ss = SubsetSum { items: &items, idx: 0, chosen: vec![], sum: 0, target: 30 };
    let mut all = BTreeSet::new();
    let _ = ss.search(&mut |s: &SubsetSum| {
        let mut c = s.chosen.clone();
        c.sort_unstable();
        all.insert(c);
        ControlFlow::<()>::Continue(())
    });
    println!("subsets summing to 30: {:?}", all);
    let found = ss.search(&mut |s| if s.chosen.len() == 2 { ControlFlow::Break(s.chosen.clone()) } else { ControlFlow::Continue(()) });
    println!("first pair: {:?}", found);
    let total: ControlFlow<i32, i32> = items.iter().try_fold(0i32, |acc, &x| if acc > 50 { ControlFlow::Break(acc) } else { ControlFlow::Continue(acc + x as i32) });
    println!("{:?} {:?}", total, total.break_value());
}
