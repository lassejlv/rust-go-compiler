// Chess move generation with bitboards: FEN parsing, pseudo-legal move
// generation (sliders by ray scanning, castling, en passant, promotions),
// make/unmake via copy, legality by king-attack test, perft node counts
// checked against known values, a material+mobility evaluator, and a tiny
// alpha-beta search with move ordering.
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

const PAWN: usize = 0;
const KNIGHT: usize = 1;
const BISHOP: usize = 2;
const ROOK: usize = 3;
const QUEEN: usize = 4;
const KING: usize = 5;
const PIECE_CHARS: [char; 6] = ['p', 'n', 'b', 'r', 'q', 'k'];

#[derive(Clone)]
struct Board {
    bb: [[u64; 6]; 2],
    side: Color,
    castling: u8, // KQkq bits
    ep: Option<u8>,
    halfmove: u32,
    fullmove: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Move {
    from: u8,
    to: u8,
    promo: Option<usize>,
    flag: u8, // 1 = double push, 2 = en passant, 3 = castle
}

fn sq_name(s: u8) -> String {
    format!("{}{}", (b'a' + s % 8) as char, s / 8 + 1)
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", sq_name(self.from), sq_name(self.to))?;
        if let Some(p) = self.promo {
            write!(f, "{}", PIECE_CHARS[p])?;
        }
        Ok(())
    }
}

const KNIGHT_D: [(i8, i8); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
const KING_D: [(i8, i8); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const ROOK_D: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_D: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

fn offset(s: u8, (df, dr): (i8, i8)) -> Option<u8> {
    let f = (s % 8) as i8 + df;
    let r = (s / 8) as i8 + dr;
    if (0..8).contains(&f) && (0..8).contains(&r) {
        Some((r * 8 + f) as u8)
    } else {
        None
    }
}

struct Tables {
    knight: [u64; 64],
    king: [u64; 64],
}

impl Tables {
    fn new() -> Tables {
        let mut t = Tables { knight: [0; 64], king: [0; 64] };
        for s in 0..64u8 {
            for d in KNIGHT_D {
                if let Some(x) = offset(s, d) {
                    t.knight[s as usize] |= 1 << x;
                }
            }
            for d in KING_D {
                if let Some(x) = offset(s, d) {
                    t.king[s as usize] |= 1 << x;
                }
            }
        }
        t
    }
}

fn bits(mut b: u64) -> impl Iterator<Item = u8> {
    std::iter::from_fn(move || {
        if b == 0 {
            None
        } else {
            let s = b.trailing_zeros() as u8;
            b &= b - 1;
            Some(s)
        }
    })
}

impl Board {
    fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("FEN needs at least 4 fields, got {}", parts.len()));
        }
        let mut b = Board { bb: [[0; 6]; 2], side: Color::White, castling: 0, ep: None, halfmove: 0, fullmove: 1 };
        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len() != 8 {
            return Err("FEN board must have 8 ranks".into());
        }
        for (i, rank) in ranks.iter().enumerate() {
            let r = 7 - i as u8;
            let mut f = 0u8;
            for c in rank.chars() {
                if let Some(d) = c.to_digit(10) {
                    f += d as u8;
                    continue;
                }
                let color = if c.is_uppercase() { 0 } else { 1 };
                let p = PIECE_CHARS.iter().position(|&pc| pc == c.to_ascii_lowercase()).ok_or_else(|| format!("bad piece {:?}", c))?;
                if f >= 8 {
                    return Err(format!("rank {} too long", r + 1));
                }
                b.bb[color][p] |= 1u64 << (r * 8 + f);
                f += 1;
            }
            if f != 8 {
                return Err(format!("rank {} has {} files", r + 1, f));
            }
        }
        b.side = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            s => return Err(format!("bad side {:?}", s)),
        };
        for c in parts[2].chars() {
            b.castling |= match c {
                'K' => 1,
                'Q' => 2,
                'k' => 4,
                'q' => 8,
                '-' => 0,
                _ => return Err(format!("bad castling {:?}", c)),
            };
        }
        if parts[3] != "-" {
            let bs = parts[3].as_bytes();
            b.ep = Some((bs[1] - b'1') * 8 + (bs[0] - b'a'));
        }
        b.halfmove = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        b.fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);
        Ok(b)
    }

    fn occ(&self, c: Color) -> u64 {
        self.bb[c as usize].iter().fold(0, |a, b| a | b)
    }
    fn all(&self) -> u64 {
        self.occ(Color::White) | self.occ(Color::Black)
    }
    fn piece_at(&self, s: u8) -> Option<(Color, usize)> {
        for c in [Color::White, Color::Black] {
            for p in 0..6 {
                if self.bb[c as usize][p] >> s & 1 == 1 {
                    return Some((c, p));
                }
            }
        }
        None
    }

    fn ray_attacks(&self, s: u8, dirs: &[(i8, i8)]) -> u64 {
        let all = self.all();
        let mut out = 0;
        for &d in dirs {
            let mut cur = s;
            while let Some(n) = offset(cur, d) {
                out |= 1 << n;
                if all >> n & 1 == 1 {
                    break;
                }
                cur = n;
            }
        }
        out
    }

    fn attacked(&self, s: u8, by: Color, t: &Tables) -> bool {
        let b = &self.bb[by as usize];
        if t.knight[s as usize] & b[KNIGHT] != 0 || t.king[s as usize] & b[KING] != 0 {
            return true;
        }
        let pawn_dirs: [(i8, i8); 2] = if by == Color::White { [(-1, -1), (1, -1)] } else { [(-1, 1), (1, 1)] };
        for d in pawn_dirs {
            if let Some(x) = offset(s, d) {
                if b[PAWN] >> x & 1 == 1 {
                    return true;
                }
            }
        }
        self.ray_attacks(s, &ROOK_D) & (b[ROOK] | b[QUEEN]) != 0 || self.ray_attacks(s, &BISHOP_D) & (b[BISHOP] | b[QUEEN]) != 0
    }

    fn pseudo_moves(&self, t: &Tables) -> Vec<Move> {
        let us = self.side;
        let own = self.occ(us);
        let their = self.occ(us.other());
        let all = own | their;
        let mut mv = Vec::with_capacity(48);
        let b = &self.bb[us as usize];
        let push = |mv: &mut Vec<Move>, from: u8, targets: u64| {
            for to in bits(targets & !own) {
                mv.push(Move { from, to, promo: None, flag: 0 });
            }
        };
        for s in bits(b[KNIGHT]) {
            push(&mut mv, s, t.knight[s as usize]);
        }
        for s in bits(b[BISHOP]) {
            push(&mut mv, s, self.ray_attacks(s, &BISHOP_D));
        }
        for s in bits(b[ROOK]) {
            push(&mut mv, s, self.ray_attacks(s, &ROOK_D));
        }
        for s in bits(b[QUEEN]) {
            push(&mut mv, s, self.ray_attacks(s, &ROOK_D) | self.ray_attacks(s, &BISHOP_D));
        }
        for s in bits(b[KING]) {
            push(&mut mv, s, t.king[s as usize]);
        }
        let (dir, start_rank, promo_rank): (i8, u8, u8) = if us == Color::White { (1, 1, 7) } else { (-1, 6, 0) };
        let add_pawn = |mv: &mut Vec<Move>, from: u8, to: u8, flag: u8| {
            if to / 8 == promo_rank {
                for p in [QUEEN, ROOK, BISHOP, KNIGHT] {
                    mv.push(Move { from, to, promo: Some(p), flag });
                }
            } else {
                mv.push(Move { from, to, promo: None, flag });
            }
        };
        for s in bits(b[PAWN]) {
            if let Some(one) = offset(s, (0, dir)) {
                if all >> one & 1 == 0 {
                    add_pawn(&mut mv, s, one, 0);
                    if s / 8 == start_rank {
                        let two = offset(one, (0, dir)).unwrap();
                        if all >> two & 1 == 0 {
                            mv.push(Move { from: s, to: two, promo: None, flag: 1 });
                        }
                    }
                }
            }
            for df in [-1, 1] {
                if let Some(x) = offset(s, (df, dir)) {
                    if their >> x & 1 == 1 {
                        add_pawn(&mut mv, s, x, 0);
                    } else if Some(x) == self.ep {
                        mv.push(Move { from: s, to: x, promo: None, flag: 2 });
                    }
                }
            }
        }
        // castling
        let (rights, rank) = if us == Color::White { (self.castling & 3, 0u8) } else { (self.castling >> 2 & 3, 56u8) };
        let king_sq = rank + 4;
        if b[KING] >> king_sq & 1 == 1 && !self.attacked(king_sq, us.other(), t) {
            if rights & 1 != 0 && all & (0b0110_0000u64 << rank) == 0 && b[ROOK] >> (rank + 7) & 1 == 1 && !self.attacked(king_sq + 1, us.other(), t) {
                mv.push(Move { from: king_sq, to: king_sq + 2, promo: None, flag: 3 });
            }
            if rights & 2 != 0 && all & (0b0000_1110u64 << rank) == 0 && b[ROOK] >> rank & 1 == 1 && !self.attacked(king_sq - 1, us.other(), t) {
                mv.push(Move { from: king_sq, to: king_sq - 2, promo: None, flag: 3 });
            }
        }
        mv
    }

    fn make(&self, m: Move) -> Board {
        let mut n = self.clone();
        let us = self.side as usize;
        let them = 1 - us;
        let (_, piece) = self.piece_at(m.from).expect("no piece on from-square");
        let from_bit = 1u64 << m.from;
        let to_bit = 1u64 << m.to;
        n.bb[us][piece] &= !from_bit;
        let mut capture = false;
        for p in 0..6 {
            if n.bb[them][p] & to_bit != 0 {
                n.bb[them][p] &= !to_bit;
                capture = true;
            }
        }
        n.bb[us][m.promo.unwrap_or(piece)] |= to_bit;
        if m.flag == 2 {
            let cap = if self.side == Color::White { m.to - 8 } else { m.to + 8 };
            n.bb[them][PAWN] &= !(1u64 << cap);
            capture = true;
        }
        if m.flag == 3 {
            let (rf, rt) = if m.to > m.from { (m.from + 3, m.from + 1) } else { (m.from - 4, m.from - 1) };
            n.bb[us][ROOK] &= !(1u64 << rf);
            n.bb[us][ROOK] |= 1u64 << rt;
        }
        n.ep = if m.flag == 1 { Some((m.from + m.to) / 2) } else { None };
        for sq in [m.from, m.to] {
            n.castling &= match sq {
                0 => !2,
                7 => !1,
                4 => !3,
                56 => !8,
                63 => !4,
                60 => !12,
                _ => 0xff,
            };
        }
        n.halfmove = if piece == PAWN || capture { 0 } else { self.halfmove + 1 };
        if self.side == Color::Black {
            n.fullmove += 1;
        }
        n.side = self.side.other();
        n
    }

    fn legal_moves(&self, t: &Tables) -> Vec<(Move, Board)> {
        self.pseudo_moves(t)
            .into_iter()
            .filter_map(|m| {
                let n = self.make(m);
                let king = n.bb[self.side as usize][KING].trailing_zeros() as u8;
                if n.attacked(king, n.side, t) {
                    None
                } else {
                    Some((m, n))
                }
            })
            .collect()
    }

    fn evaluate(&self, t: &Tables) -> i32 {
        const VAL: [i32; 6] = [100, 320, 330, 500, 900, 0];
        let mut score = 0;
        for (ci, sign) in [(0usize, 1i32), (1, -1)] {
            for p in 0..6 {
                score += sign * VAL[p] * self.bb[ci][p].count_ones() as i32;
            }
        }
        let mob = self.pseudo_moves(t).len() as i32;
        let mut flipped = self.clone();
        flipped.side = self.side.other();
        flipped.ep = None;
        let opp_mob = flipped.pseudo_moves(t).len() as i32;
        let rel = if self.side == Color::White { score } else { -score };
        rel + 2 * (mob - opp_mob)
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for r in (0..8).rev() {
            for fl in 0..8 {
                let c = match self.piece_at(r * 8 + fl) {
                    Some((Color::White, p)) => PIECE_CHARS[p].to_ascii_uppercase(),
                    Some((Color::Black, p)) => PIECE_CHARS[p],
                    None => '.',
                };
                write!(f, "{}", c)?;
            }
            writeln!(f)?;
        }
        write!(f, "{:?} to move, castling {:04b}, ep {:?}", self.side, self.castling, self.ep.map(sq_name))
    }
}

fn perft(b: &Board, depth: u32, t: &Tables) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = b.legal_moves(t);
    if depth == 1 {
        return moves.len() as u64;
    }
    moves.iter().map(|(_, n)| perft(n, depth - 1, t)).sum()
}

fn alphabeta(b: &Board, depth: u32, mut alpha: i32, beta: i32, t: &Tables, nodes: &mut u64) -> (i32, Option<Move>) {
    *nodes += 1;
    let mut moves = b.legal_moves(t);
    if moves.is_empty() {
        let king = b.bb[b.side as usize][KING].trailing_zeros() as u8;
        return (if b.attacked(king, b.side.other(), t) { -100_000 - depth as i32 } else { 0 }, None);
    }
    if depth == 0 {
        return (b.evaluate(t), None);
    }
    // captures first (MVV), then promotions
    moves.sort_by_key(|(m, _)| {
        let victim = b.piece_at(m.to).map_or(-1, |(_, p)| p as i32);
        std::cmp::Reverse((victim, m.promo.is_some(), std::cmp::Reverse(m.from), m.to))
    });
    let mut best = None;
    for (m, n) in &moves {
        let (s, _) = alphabeta(n, depth - 1, -beta, -alpha, t, nodes);
        let s = -s;
        if s > alpha || best.is_none() {
            if s > alpha {
                alpha = s;
            }
            best = Some(*m);
        }
        if alpha >= beta {
            break;
        }
    }
    (alpha, best)
}

fn main() {
    let t = Tables::new();
    let positions = [
        ("start", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", vec![20u64, 400, 8902]),
        ("kiwipete", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", vec![48, 2039]),
        ("pos3", "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", vec![14, 191, 2812]),
        ("pos4", "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", vec![6, 264]),
        ("pos5", "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", vec![44, 1486]),
    ];
    for (name, fen, expected) in &positions {
        let b = Board::from_fen(fen).unwrap();
        let counts: Vec<u64> = (1..=expected.len() as u32).map(|d| perft(&b, d, &t)).collect();
        println!("{:<9} perft {:?} {}", name, counts, if &counts == expected { "OK" } else { "MISMATCH" });
    }
    let b = Board::from_fen(positions[1].1).unwrap();
    println!("{}", b);
    let mut divide: Vec<(String, u64)> = b.legal_moves(&t).iter().map(|(m, n)| (m.to_string(), perft(n, 1, &t))).collect();
    divide.sort();
    println!("divide: {}", divide.iter().take(8).map(|(m, c)| format!("{}:{}", m, c)).collect::<Vec<_>>().join(" "));
    for bad in ["8/8/8 w - -", "rnbqkbnr/ppppxppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -", "rnbqkbnr/pppppppp/9/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -", "8/8/8/8/8/8/8/8 x - -", "8/8/8/8/8/8/8/8 w"] {
        println!("{:?}", Board::from_fen(bad).err());
    }
    // mate in one and a tactic
    for (fen, depth) in [("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1", 2), ("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4", 2), ("7k/8/6Q1/8/8/8/8/K7 w - - 0 1", 1)] {
        let b = Board::from_fen(fen).unwrap();
        let mut nodes = 0;
        let (score, mv) = alphabeta(&b, depth, -1_000_000, 1_000_000, &t, &mut nodes);
        println!("best {} score {} nodes {}", mv.map_or("-".into(), |m| m.to_string()), score, nodes);
    }
    // play a few greedy moves from the start
    let mut b = Board::from_fen(positions[0].1).unwrap();
    let mut line = vec![];
    for _ in 0..6 {
        let mut nodes = 0;
        let (_, mv) = alphabeta(&b, 2, -1_000_000, 1_000_000, &t, &mut nodes);
        let m = mv.unwrap();
        line.push(m.to_string());
        b = b.make(m);
    }
    println!("line {} -> fullmove {} halfmove {}", line.join(" "), b.fullmove, b.halfmove);
}
