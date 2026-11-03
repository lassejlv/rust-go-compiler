// Sudoku solver using bitmasks for rows/cols/boxes, minimum-remaining-value
// heuristic, backtracking with a search counter, and puzzle validation.
#[derive(Clone)]
struct Board {
    cells: [u8; 81],
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
}

#[derive(Debug)]
enum ParseError {
    BadLength(usize),
    BadChar(char, usize),
    Conflict { idx: usize, digit: u8 },
}

const fn box_of(i: usize) -> usize { (i / 27) * 3 + (i % 9) / 3 }

impl Board {
    fn parse(s: &str) -> Result<Board, ParseError> {
        let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.len() != 81 {
            return Err(ParseError::BadLength(chars.len()));
        }
        let mut b = Board { cells: [0; 81], rows: [0; 9], cols: [0; 9], boxes: [0; 9] };
        for (i, &c) in chars.iter().enumerate() {
            let d = match c {
                '.' | '0' => 0,
                '1'..='9' => c as u8 - b'0',
                _ => return Err(ParseError::BadChar(c, i)),
            };
            if d != 0 {
                if !b.can_place(i, d) {
                    return Err(ParseError::Conflict { idx: i, digit: d });
                }
                b.place(i, d);
            }
        }
        Ok(b)
    }
    fn can_place(&self, i: usize, d: u8) -> bool {
        let bit = 1u16 << d;
        (self.rows[i / 9] | self.cols[i % 9] | self.boxes[box_of(i)]) & bit == 0
    }
    fn place(&mut self, i: usize, d: u8) {
        let bit = 1u16 << d;
        self.cells[i] = d;
        self.rows[i / 9] |= bit;
        self.cols[i % 9] |= bit;
        self.boxes[box_of(i)] |= bit;
    }
    fn unplace(&mut self, i: usize) {
        let bit = !(1u16 << self.cells[i]);
        self.rows[i / 9] &= bit;
        self.cols[i % 9] &= bit;
        self.boxes[box_of(i)] &= bit;
        self.cells[i] = 0;
    }
    fn candidates(&self, i: usize) -> u16 {
        !(self.rows[i / 9] | self.cols[i % 9] | self.boxes[box_of(i)]) & 0b11_1111_1110
    }
    fn solve(&mut self, steps: &mut u64) -> bool {
        *steps += 1;
        let mut best: Option<(usize, u16)> = None;
        for i in 0..81 {
            if self.cells[i] != 0 { continue; }
            let c = self.candidates(i);
            let n = c.count_ones();
            if n == 0 { return false; }
            if best.map_or(true, |(_, bc)| n < bc.count_ones()) {
                best = Some((i, c));
                if n == 1 { break; }
            }
        }
        let Some((i, mut c)) = best else { return true };
        while c != 0 {
            let d = c.trailing_zeros() as u8;
            c &= c - 1;
            self.place(i, d);
            if self.solve(steps) { return true; }
            self.unplace(i);
        }
        false
    }
    fn count_solutions(&mut self, limit: usize) -> usize {
        let Some(i) = (0..81).find(|&i| self.cells[i] == 0) else { return 1 };
        let mut c = self.candidates(i);
        let mut total = 0;
        while c != 0 && total < limit {
            let d = c.trailing_zeros() as u8;
            c &= c - 1;
            self.place(i, d);
            total += self.count_solutions(limit - total);
            self.unplace(i);
        }
        total
    }
    fn is_valid_solution(&self) -> bool {
        self.rows.iter().chain(&self.cols).chain(&self.boxes).all(|&m| m == 0b11_1111_1110)
    }
    fn print(&self) {
        for r in 0..9 {
            if r % 3 == 0 && r > 0 { println!("------+-------+------"); }
            let line: Vec<String> = (0..9)
                .map(|c| {
                    let d = self.cells[r * 9 + c];
                    let s = if d == 0 { ".".to_string() } else { d.to_string() };
                    if c % 3 == 2 && c < 8 { format!("{} |", s) } else { s }
                })
                .collect();
            println!("{}", line.join(" "));
        }
    }
}

fn main() {
    let puzzles = [
        "53..7....6..195....98....6.8...6...34..8.3..17...2...6.6....28....419..5....8..79",
        "..9748...7.........2.1.9.....7...24..64.1.59..98...3.....8.3.2.........6...2759..",
        "8..........36......7..9.2...5...7.......457.....1...3...1....68..85...1..9....4..",
    ];
    for (n, p) in puzzles.iter().enumerate() {
        let mut b = Board::parse(p).unwrap();
        let givens = b.cells.iter().filter(|&&c| c != 0).count();
        let mut steps = 0;
        let ok = b.solve(&mut steps);
        println!("puzzle {} givens={} solved={} valid={} steps>0={}", n, givens, ok, b.is_valid_solution(), steps > 0);
        if n < 2 { b.print(); } else {
            println!("first row: {:?}", &b.cells[..9]);
        }
    }
    let mut amb = Board::parse(&puzzles[0].replacen('5', ".", 1).replacen('3', ".", 1).replacen('7', ".", 1)).unwrap();
    println!("solutions (relaxed, cap 10): {}", amb.count_solutions(10));
    let mut uniq = Board::parse(puzzles[0]).unwrap();
    println!("solutions (original): {}", uniq.count_solutions(10));
    for bad in ["123", &format!("x{}", &puzzles[0][1..]), &format!("55{}", &puzzles[0][2..])] {
        match Board::parse(bad) {
            Ok(_) => println!("parsed?"),
            Err(e) => println!("error: {:?}", e),
        }
    }
    let empty = Board::parse(&".".repeat(81)).unwrap();
    println!("candidates of empty cell: {:#011b} ({} options)", empty.candidates(40), empty.candidates(40).count_ones());
    println!("box_of: {:?}", [0, 8, 30, 40, 80].map(box_of));
}
