// Pattern matching with constants (incl. struct/tuple/str/assoc consts),
// @ bindings with subpatterns, nested slices binding sub-slices, box-free
// deref patterns via match ergonomics, ranges over chars/bytes/negatives,
// or-patterns binding the same names, and matches! with guards.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Pt { x: i32, y: i32 }

const ORIGIN: Pt = Pt { x: 0, y: 0 };
const UNIT_X: Pt = Pt { x: 1, y: 0 };
const KEYWORDS: (&str, &str) = ("fn", "let");
const LIMIT: i64 = -10;

#[derive(Debug)]
enum Tree { Leaf(i32), Node(Box<Tree>, Box<Tree>), Tagged { tag: &'static str, kids: Vec<Tree> } }

struct Bounds;
impl Bounds { const LO: u8 = 10; const HI: u8 = 20; }

fn classify_pt(p: Pt) -> String {
    match p {
        ORIGIN => "origin".into(),
        UNIT_X => "unit-x".into(),
        Pt { x: 0, y } | Pt { x: y, y: 0 } => format!("on-axis {}", y),
        Pt { x, y } if x == y => format!("diagonal {}", x),
        Pt { x: x @ (-5..=5), y: y @ (-5..=5) } => format!("near ({},{})", x, y),
        _ => "far".into(),
    }
}

fn kw(s: &str) -> &'static str {
    match s {
        "fn" => "function keyword",
        x if x == KEYWORDS.1 => "binding keyword",
        "" => "empty",
        _ if s.starts_with('_') => "private-ish",
        _ => "ident",
    }
}

fn tree_sum(t: &Tree) -> i32 {
    match t {
        Tree::Leaf(n @ 0..=9) => *n,
        Tree::Leaf(n) => n / 10,
        Tree::Node(l, r) => match (&**l, &**r) {
            (Tree::Leaf(a), Tree::Leaf(b)) if a == b => 100 + a,
            (l, r) => tree_sum(l) + tree_sum(r),
        },
        Tree::Tagged { tag: "skip", .. } => 0,
        Tree::Tagged { tag, kids } => {
            let s: i32 = kids.iter().map(tree_sum).sum();
            s * tag.len() as i32
        }
    }
}

fn slices(v: &[i32]) -> String {
    match v {
        [] => "empty".into(),
        [x] => format!("one {}", x),
        [first, .., last] if first == last => format!("bookends {}", first),
        [a, b, rest @ ..] if rest.len() >= 3 => format!("{} {} then {} more (sum {})", a, b, rest.len(), rest.iter().sum::<i32>()),
        [.., penult @ 4..=6, _] => format!("penultimate {}", penult),
        [head @ .., tail] => format!("head {:?} tail {}", head, tail),
    }
}

fn byte_class(b: u8) -> &'static str {
    match b {
        b'0'..=b'9' => "digit",
        b'a'..=b'f' | b'A'..=b'F' => "hex-letter",
        b'g'..=b'z' | b'G'..=b'Z' => "letter",
        Bounds::LO..=Bounds::HI => "low-control",
        0x80.. => "high",
        _ => "other",
    }
}

fn temps(t: i64) -> &'static str {
    match t {
        i64::MIN..LIMIT => "frigid",
        LIMIT..=0 => "freezing",
        1..=15 => "cold",
        16..=25 => "mild",
        _ => "hot",
    }
}

fn opt_ref(o: &Option<(String, Vec<u8>)>) -> String {
    match o {
        Some((name, bytes)) if bytes.is_empty() => format!("{} (no bytes)", name),
        Some((name, bytes)) => match bytes.as_slice() {
            [0xFF, ..] => format!("{} starts with FF", name),
            [.., last] => format!("{} ends with {}", name, last),
            [] => unreachable!(),
        },
        None => "nothing".into(),
    }
}

fn main() {
    for p in [ORIGIN, UNIT_X, Pt { x: 0, y: 7 }, Pt { x: -3, y: 0 }, Pt { x: 4, y: 4 }, Pt { x: 2, y: -5 }, Pt { x: 9, y: 1 }] {
        println!("{:?} -> {}", p, classify_pt(p));
    }
    for s in ["fn", "let", "", "_x", "foo"] { print!("{:?}={} | ", s, kw(s)); }
    println!();
    let t = Tree::Node(
        Box::new(Tree::Tagged { tag: "abc", kids: vec![Tree::Leaf(3), Tree::Leaf(40), Tree::Node(Box::new(Tree::Leaf(5)), Box::new(Tree::Leaf(5)))] }),
        Box::new(Tree::Node(Box::new(Tree::Tagged { tag: "skip", kids: vec![Tree::Leaf(1)] }), Box::new(Tree::Leaf(99)))),
    );
    println!("tree sum {}", tree_sum(&t));
    if let Tree::Node(l, _) = &t {
        if let Tree::Tagged { kids, .. } = l.as_ref() {
            println!("first tagged kids: {:?}", kids.iter().map(|k| matches!(k, Tree::Leaf(n) if *n < 10)).collect::<Vec<_>>());
        }
    }
    for v in [&[][..], &[5], &[3, 9, 3], &[1, 2, 3, 4, 5, 6], &[1, 5, 0], &[7, 8], &[1, 2, 3, 4]] {
        println!("{:?} -> {}", v, slices(v));
    }
    let bytes: Vec<&str> = [b'7', b'c', b'Q', 15, 200, b' '].iter().map(|&b| byte_class(b)).collect();
    println!("{:?}", bytes);
    println!("{:?}", [-40, -10, -3, 0, 7, 20, 30].map(temps));
    let opts = [None, Some(("a".to_string(), vec![])), Some(("b".to_string(), vec![0xFF, 1])), Some(("c".to_string(), vec![1, 2, 42]))];
    for o in &opts { println!("{}", opt_ref(o)); }
    let pairs = [(1, 'a'), (2, 'b'), (3, 'c')];
    for &(n, c) in pairs.iter().filter(|(n, _)| n % 2 == 1) { print!("{}{} ", n, c); }
    println!();
    let mut grid = [[0u8; 3]; 2];
    for (r, row) in grid.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() { *cell = (r * 3 + c) as u8; }
    }
    let [[a, _, b], [.., z]] = grid;
    println!("grid corners {} {} {}", a, b, z);
    let msg @ ("hi" | "hello") = "hello" else { panic!() };
    println!("let-else with @ or-pattern: {}", msg);
    let n = 42u32;
    let desc = match n { x @ (0 | 42 | 100) => format!("special {}", x), x => format!("plain {}", x) };
    println!("{}", desc);
    let nested = Some(Some(Pt { x: 3, y: 4 }));
    if let Some(Some(Pt { x: px @ 1..=5, .. })) = nested { println!("nested bind px={}", px); }
    let refs = vec![&1, &2, &3];
    let total: i32 = refs.iter().map(|&&x| x).sum();
    println!("double ref pattern sum {}", total);
    let mut pair = (String::from("left"), 5);
    let (ref mut s, ref mut k) = pair;
    s.push('!');
    *k += 1;
    println!("ref mut tuple {:?}", pair);
    let c = 'm';
    println!("{}", match c { 'a'..='l' => 1, 'm' | 'n' => 2, 'o'..='z' => 3, _ => 0 });
}
