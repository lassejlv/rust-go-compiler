// Chess move generator (mailbox 10x12) with castling, en passant and
// promotions; verified by perft on known positions. Also FEN round-trip,
// SAN-ish move printing and a tiny material-only 2-ply search.
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Color { White, Black }

impl Color {
    fn other(self) -> Color { match self { Color::White => Color::Black, Color::Black => Color::White } }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind { Pawn, Knight, Bishop, Rook, Queen, King }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sq { Off, Empty, P(Color, Kind) }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Move { from: u8, to: u8, promo: Option<Kind>, flag: Flag }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Flag { Normal, DoublePush, EnPassant, CastleK, CastleQ }

#[derive(Clone)]
struct Board {
    sq: [Sq; 120],
    side: Color,
    castle: u8, // 1=K 2=Q 4=k 8=q
    ep: Option<u8>,
    halfmove: u32,
    fullmove: u32,
}

const N: i32 = -10;
const S: i32 = 10;
const E: i32 = 1;
const W: i32 = -1;
const KNIGHT: [i32; 8] = [-21, -19, -12, -8, 8, 12, 19, 21];
const KING: [i32; 8] = [N, S, E, W, N + E, N + W, S + E, S + W];
const ROOK: [i32; 4] = [N, S, E, W];
const BISHOP: [i32; 4] = [N + E, N + W, S + E, S + W];

fn idx(file: usize, rank: usize) -> u8 { (21 + file + (7 - rank) * 10) as u8 }
fn name(i: u8) -> String {
    let f = (i % 10 - 1) as u8;
    let r = 7 - (i / 10 - 2);
    format!("{}{}", (b'a' + f) as char, r + 1)
}

impl Board {
    fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 { return Err("too few fields".into()); }
        let mut sq = [Sq::Off; 120];
        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len() != 8 { return Err(format!("expected 8 ranks, got {}", ranks.len())); }
        for (ri, row) in ranks.iter().enumerate() {
            let rank = 7 - ri;
            let mut file = 0;
            for c in row.chars() {
                if let Some(d) = c.to_digit(10) {
                    for _ in 0..d { sq[idx(file, rank) as usize] = Sq::Empty; file += 1; }
                    continue;
                }
                let color = if c.is_ascii_uppercase() { Color::White } else { Color::Black };
                let kind = match c.to_ascii_lowercase() {
                    'p' => Kind::Pawn, 'n' => Kind::Knight, 'b' => Kind::Bishop,
                    'r' => Kind::Rook, 'q' => Kind::Queen, 'k' => Kind::King,
                    _ => return Err(format!("bad piece {:?}", c)),
                };
                if file >= 8 { return Err("rank overflow".into()); }
                sq[idx(file, rank) as usize] = Sq::P(color, kind);
                file += 1;
            }
            if file != 8 { return Err(format!("rank {} has {} files", rank + 1, file)); }
        }
        let side = match parts[1] { "w" => Color::White, "b" => Color::Black, s => return Err(format!("bad side {}", s)) };
        let mut castle = 0;
        for c in parts[2].chars() {
            castle |= match c { 'K' => 1, 'Q' => 2, 'k' => 4, 'q' => 8, '-' => 0, _ => return Err("bad castling".into()) };
        }
        let ep = if parts[3] == "-" { None } else {
            let b = parts[3].as_bytes();
            Some(idx((b[0] - b'a') as usize, (b[1] - b'1') as usize))
        };
        let halfmove = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
        Ok(Board { sq, side, castle, ep, halfmove, fullmove })
    }

    fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                match self.sq[idx(file, rank) as usize] {
                    Sq::P(c, k) => {
                        if empty > 0 { s.push_str(&empty.to_string()); empty = 0; }
                        let ch = match k { Kind::Pawn => 'p', Kind::Knight => 'n', Kind::Bishop => 'b', Kind::Rook => 'r', Kind::Queen => 'q', Kind::King => 'k' };
                        s.push(if c == Color::White { ch.to_ascii_uppercase() } else { ch });
                    }
                    _ => empty += 1,
                }
            }
            if empty > 0 { s.push_str(&empty.to_string()); }
            if rank > 0 { s.push('/'); }
        }
        let cs: String = [(1, 'K'), (2, 'Q'), (4, 'k'), (8, 'q')].iter().filter(|(b, _)| self.castle & b != 0).map(|(_, c)| *c).collect();
        format!("{} {} {} {} {} {}", s, if self.side == Color::White { 'w' } else { 'b' },
            if cs.is_empty() { "-".to_string() } else { cs },
            self.ep.map_or("-".to_string(), name), self.halfmove, self.fullmove)
    }

    fn king(&self, c: Color) -> u8 {
        (0..120).find(|&i| self.sq[i] == Sq::P(c, Kind::King)).unwrap() as u8
    }

    fn attacked(&self, t: u8, by: Color) -> bool {
        let t = t as i32;
        let at = |d: i32| self.sq[(t + d) as usize];
        // pawns
        let pd = if by == Color::White { [S + E, S + W] } else { [N + E, N + W] };
        if pd.iter().any(|&d| at(d) == Sq::P(by, Kind::Pawn)) { return true; }
        if KNIGHT.iter().any(|&d| at(d) == Sq::P(by, Kind::Knight)) { return true; }
        if KING.iter().any(|&d| at(d) == Sq::P(by, Kind::King)) { return true; }
        for (dirs, k) in [(&ROOK, Kind::Rook), (&BISHOP, Kind::Bishop)] {
            for &d in dirs.iter() {
                let mut p = t + d;
                loop {
                    match self.sq[p as usize] {
                        Sq::Empty => p += d,
                        Sq::P(c, kk) if c == by && (kk == k || kk == Kind::Queen) => return true,
                        _ => break,
                    }
                }
            }
        }
        false
    }

    fn pseudo_moves(&self, out: &mut Vec<Move>) {
        let us = self.side;
        for from in 21..99u8 {
            let k = match self.sq[from as usize] { Sq::P(c, k) if c == us => k, _ => continue };
            let f = from as i32;
            let mut push = |to: i32, flag: Flag, out: &mut Vec<Move>| out.push(Move { from, to: to as u8, promo: None, flag });
            match k {
                Kind::Pawn => {
                    let (fwd, start_rank, promo_rank) = if us == Color::White { (N, 1, 7) } else { (S, 6, 0) };
                    let rank_of = |i: i32| 7 - (i / 10 - 2);
                    let add = |to: i32, flag: Flag, out: &mut Vec<Move>| {
                        if rank_of(to) == promo_rank {
                            for p in [Kind::Queen, Kind::Rook, Kind::Bishop, Kind::Knight] {
                                out.push(Move { from, to: to as u8, promo: Some(p), flag });
                            }
                        } else {
                            out.push(Move { from, to: to as u8, promo: None, flag });
                        }
                    };
                    if self.sq[(f + fwd) as usize] == Sq::Empty {
                        add(f + fwd, Flag::Normal, out);
                        if rank_of(f) == start_rank && self.sq[(f + 2 * fwd) as usize] == Sq::Empty {
                            out.push(Move { from, to: (f + 2 * fwd) as u8, promo: None, flag: Flag::DoublePush });
                        }
                    }
                    for d in [fwd + E, fwd + W] {
                        let t = f + d;
                        match self.sq[t as usize] {
                            Sq::P(c, _) if c != us => add(t, Flag::Normal, out),
                            Sq::Empty if self.ep == Some(t as u8) => add(t, Flag::EnPassant, out),
                            _ => {}
                        }
                    }
                }
                Kind::Knight | Kind::King => {
                    let dirs: &[i32] = if k == Kind::Knight { &KNIGHT } else { &KING };
                    for &d in dirs {
                        match self.sq[(f + d) as usize] {
                            Sq::Empty => push(f + d, Flag::Normal, out),
                            Sq::P(c, _) if c != us => push(f + d, Flag::Normal, out),
                            _ => {}
                        }
                    }
                    if k == Kind::King {
                        let (kbit, qbit, home) = if us == Color::White { (1, 2, 95) } else { (4, 8, 25) };
                        let them = us.other();
                        if f == home && self.castle & kbit != 0 && self.sq[96 - 70 * (us == Color::Black) as usize] == Sq::Empty
                            && self.sq[(home + 2) as usize] == Sq::Empty
                            && !self.attacked(home as u8, them) && !self.attacked((home + 1) as u8, them) && !self.attacked((home + 2) as u8, them) {
                            push(home + 2, Flag::CastleK, out);
                        }
                        if f == home && self.castle & qbit != 0 && self.sq[(home - 1) as usize] == Sq::Empty
                            && self.sq[(home - 2) as usize] == Sq::Empty && self.sq[(home - 3) as usize] == Sq::Empty
                            && !self.attacked(home as u8, them) && !self.attacked((home - 1) as u8, them) && !self.attacked((home - 2) as u8, them) {
                            push(home - 2, Flag::CastleQ, out);
                        }
                    }
                }
                _ => {
                    let dirs: &[i32] = match k { Kind::Rook => &ROOK, Kind::Bishop => &BISHOP, _ => &KING };
                    for &d in dirs {
                        let mut t = f + d;
                        loop {
                            match self.sq[t as usize] {
                                Sq::Empty => { push(t, Flag::Normal, out); t += d; }
                                Sq::P(c, _) if c != us => { push(t, Flag::Normal, out); break; }
                                _ => break,
                            }
                        }
                    }
                }
            }
        }
    }

    fn make(&mut self, m: Move) -> Sq {
        let us = self.side;
        let piece = self.sq[m.from as usize];
        let mut captured = self.sq[m.to as usize];
        self.sq[m.to as usize] = match m.promo { Some(k) => Sq::P(us, k), None => piece };
        self.sq[m.from as usize] = Sq::Empty;
        match m.flag {
            Flag::EnPassant => {
                let cap = if us == Color::White { m.to as i32 + S } else { m.to as i32 + N } as usize;
                captured = self.sq[cap];
                self.sq[cap] = Sq::Empty;
            }
            Flag::CastleK => { self.sq[(m.to - 1) as usize] = self.sq[(m.to + 1) as usize]; self.sq[(m.to + 1) as usize] = Sq::Empty; }
            Flag::CastleQ => { self.sq[(m.to + 1) as usize] = self.sq[(m.to - 2) as usize]; self.sq[(m.to - 2) as usize] = Sq::Empty; }
            _ => {}
        }
        self.ep = if m.flag == Flag::DoublePush { Some(((m.from as u16 + m.to as u16) / 2) as u8) } else { None };
        for s in [m.from, m.to] {
            self.castle &= match s { 95 => !3, 25 => !12, 98 => !1, 91 => !2, 28 => !4, 21 => !8, _ => !0 };
        }
        self.halfmove = if matches!(piece, Sq::P(_, Kind::Pawn)) || captured != Sq::Empty { 0 } else { self.halfmove + 1 };
        if us == Color::Black { self.fullmove += 1; }
        self.side = us.other();
        captured
    }

    fn legal_moves(&self) -> Vec<Move> {
        let mut pseudo = Vec::with_capacity(64);
        self.pseudo_moves(&mut pseudo);
        pseudo.into_iter().filter(|&m| {
            let mut b = self.clone();
            b.make(m);
            !b.attacked(b.king(self.side), b.side)
        }).collect()
    }

    fn perft(&self, depth: u32) -> u64 {
        if depth == 0 { return 1; }
        let moves = self.legal_moves();
        if depth == 1 { return moves.len() as u64; }
        moves.iter().map(|&m| { let mut b = self.clone(); b.make(m); b.perft(depth - 1) }).sum()
    }

    fn material(&self) -> i32 {
        self.sq.iter().map(|s| match s {
            Sq::P(c, k) => {
                let v = match k { Kind::Pawn => 100, Kind::Knight => 320, Kind::Bishop => 330, Kind::Rook => 500, Kind::Queen => 900, Kind::King => 0 };
                if *c == Color::White { v } else { -v }
            }
            _ => 0,
        }).sum()
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", name(self.from), name(self.to))?;
        if let Some(p) = self.promo {
            write!(f, "{}", match p { Kind::Queen => 'q', Kind::Rook => 'r', Kind::Bishop => 'b', _ => 'n' })?;
        }
        Ok(())
    }
}

fn negamax(b: &Board, depth: u32, mut alpha: i32, beta: i32, nodes: &mut u64) -> i32 {
    *nodes += 1;
    let sign = if b.side == Color::White { 1 } else { -1 };
    if depth == 0 { return sign * b.material(); }
    let moves = b.legal_moves();
    if moves.is_empty() {
        return if b.attacked(b.king(b.side), b.side.other()) { -100_000 } else { 0 };
    }
    let mut best = i32::MIN + 1;
    for m in moves {
        let mut c = b.clone();
        c.make(m);
        let v = -negamax(&c, depth - 1, -beta, -alpha, nodes);
        best = best.max(v);
        alpha = alpha.max(v);
        if alpha >= beta { break; }
    }
    best
}

fn main() {
    let positions = [
        ("start", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 3, &[20u64, 400, 8902][..]),
        ("kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 2, &[48, 2039][..]),
        ("pos3", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 3, &[14, 191, 2812][..]),
        ("pos4", "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 2, &[6, 264][..]),
        ("pos5", "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 2, &[44, 1486][..]),
    ];
    for (name, fen, depth, expected) in positions.iter() {
        let b = Board::from_fen(fen).unwrap();
        assert_eq!(&b.to_fen(), fen, "fen round trip");
        let mut results = Vec::new();
        for d in 1..=*depth {
            results.push(b.perft(d));
        }
        println!("{:<9} perft {:?} {}", name, results, if &results[..] == *expected { "OK" } else { "MISMATCH" });
    }

    let b = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let mut moves: Vec<String> = b.legal_moves().iter().map(|m| m.to_string()).collect();
    moves.sort();
    println!("start moves: {}", moves.join(" "));

    // play a short game and show FEN after each move
    let mut g = b.clone();
    for mv in ["e2e4", "d7d5", "e4d5", "c7c5", "d5c6", "b8c6", "g1f3", "e7e5", "f1b5", "g8f6", "e1g1", "c8g4"] {
        let m = g.legal_moves().into_iter().find(|m| m.to_string() == mv).unwrap_or_else(|| panic!("illegal {}", mv));
        let cap = g.make(m);
        println!("{:<5} {:?} {:<9} {}", mv, m.flag, if cap != Sq::Empty { "capture" } else { "" }, g.to_fen());
    }
    let promo = Board::from_fen("8/P6k/8/8/8/8/6pK/8 w - - 0 1").unwrap();
    let pm: Vec<String> = promo.legal_moves().iter().map(|m| m.to_string()).collect();
    println!("promotion moves: {:?}", pm);

    let mate_in_one = Board::from_fen("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1").unwrap();
    let mut nodes = 0;
    let mut best: Option<(i32, String)> = None;
    for m in mate_in_one.legal_moves() {
        let mut c = mate_in_one.clone();
        c.make(m);
        let v = -negamax(&c, 1, -1_000_000, 1_000_000, &mut nodes);
        if best.as_ref().map_or(true, |(bv, _)| v > *bv) { best = Some((v, m.to_string())); }
    }
    println!("best move {:?} after {} nodes", best, nodes);
    let stalemate = Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    println!("stalemate: moves={} in_check={}", stalemate.legal_moves().len(), stalemate.attacked(stalemate.king(Color::Black), Color::White));
    for bad in ["8/8/8 w - -", "rnbqkbnr/pppppppp/9/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -", "rnbqkbnr/ppppxppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -", "8/8/8/8/8/8/8/8 x - -"] {
        println!("{:?}", Board::from_fen(bad).err());
    }
}
