// Ordering and comparison utilities: derived orderings on enums/structs,
// Ordering combinators, Reverse, min/max/clamp helpers, sort stability,
// lexicographic comparisons of tuples/slices/Options/strings, PartialOrd
// for floats, and custom Ord impls used by collections.
use std::cmp::{self, Ordering, Reverse};
use std::collections::{BTreeSet, BinaryHeap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Suit { Clubs, Diamonds, Hearts, Spades }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Card { rank: u8, suit: Suit }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape { Dot, Line(u32), Rect { w: u32, h: u32 }, Poly(Vec<u8>) }

#[derive(Debug, Clone, PartialEq)]
struct Player { name: String, score: f64, level: u32 }

#[derive(Debug, Eq, PartialEq)]
struct Version { major: u32, minor: u32, patch: u32, pre: Option<String> }

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        self.major.cmp(&o.major)
            .then(self.minor.cmp(&o.minor))
            .then_with(|| self.patch.cmp(&o.patch))
            .then_with(|| match (&self.pre, &o.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => a.cmp(b),
            })
    }
}
impl PartialOrd for Version { fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) } }

fn v(s: &str) -> Version {
    let (core, pre) = match s.split_once('-') { Some((c, p)) => (c, Some(p.to_string())), None => (s, None) };
    let n: Vec<u32> = core.split('.').map(|x| x.parse().unwrap()).collect();
    Version { major: n[0], minor: n[1], patch: n[2], pre }
}

fn main() {
    println!("ordering ops: {:?} {:?} {:?} {:?}", Ordering::Less.reverse(), Ordering::Equal.then(Ordering::Greater), Ordering::Less.then(Ordering::Greater), Ordering::Equal.then_with(|| 2.cmp(&1)));
    println!("is_*: {} {} {} {} {} {}", Ordering::Less.is_lt(), Ordering::Less.is_le(), Ordering::Equal.is_ge(), Ordering::Greater.is_gt(), Ordering::Equal.is_ne(), Ordering::Equal.is_eq());
    println!("as i8: {} {} {}", Ordering::Less as i8, Ordering::Equal as i8, Ordering::Greater as i8);
    println!("cmp::min/max {} {} min_by_key {:?} max_by {:?}", cmp::min(3, 7), cmp::max("a", "b"), cmp::min_by_key(-5i32, 4, |x| x.abs()), cmp::max_by(1.5f64, -2.5, |a, b| a.abs().total_cmp(&b.abs())));
    println!("minmax ties: min({:?},{:?}) {:?} max {:?}", (1, 'a'), (1, 'b'), cmp::min_by_key((1, 'a'), (1, 'b'), |p| p.0), cmp::max_by_key((1, 'a'), (1, 'b'), |p| p.0));
    println!("clamp {} {} {:?}", 15.clamp(1, 10), "m".clamp("a", "f"), Reverse(5).clamp(Reverse(9), Reverse(7)));
    println!("Reverse cmp {:?} {}", Reverse(1).cmp(&Reverse(2)), Reverse("a") < Reverse("b"));

    let mut cards = vec![
        Card { rank: 10, suit: Suit::Hearts }, Card { rank: 2, suit: Suit::Spades }, Card { rank: 10, suit: Suit::Clubs },
        Card { rank: 14, suit: Suit::Diamonds }, Card { rank: 2, suit: Suit::Clubs },
    ];
    cards.sort();
    println!("derived sort: {:?}", cards.iter().map(|c| format!("{}{:?}", c.rank, c.suit).chars().take(3).collect::<String>()).collect::<Vec<_>>());
    cards.sort_by_key(|c| (Reverse(c.suit), c.rank));
    println!("by suit desc: {:?}", cards.iter().map(|c| (c.suit, c.rank)).collect::<Vec<_>>());
    println!("max card {:?} min {:?}", cards.iter().max(), cards.iter().min());
    println!("suit cmp {:?} {} {:?}", Suit::Clubs.cmp(&Suit::Spades), Suit::Hearts > Suit::Diamonds, Suit::Spades.max(Suit::Clubs));

    let mut shapes = vec![Shape::Poly(vec![1, 2]), Shape::Rect { w: 2, h: 1 }, Shape::Line(5), Shape::Dot, Shape::Rect { w: 1, h: 9 }, Shape::Line(2), Shape::Poly(vec![1])];
    shapes.sort();
    println!("enum variant order: {:?}", shapes);

    // stable sorting keeps insertion order among equals
    let mut players = vec![
        Player { name: "ann".into(), score: 9.5, level: 3 },
        Player { name: "bob".into(), score: 7.0, level: 5 },
        Player { name: "cy".into(), score: 9.5, level: 1 },
        Player { name: "dee".into(), score: f64::NAN, level: 3 },
        Player { name: "eve".into(), score: 7.0, level: 3 },
    ];
    players.sort_by_key(|p| p.level);
    println!("stable by level: {:?}", players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>());
    players.sort_by(|a, b| b.score.total_cmp(&a.score));
    println!("by score desc (NaN first via total_cmp): {:?}", players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>());
    players.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal).then_with(|| a.name.cmp(&b.name)));
    println!("partial_cmp fallback: {:?}", players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>());
    let best = players.iter().filter(|p| !p.score.is_nan()).max_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    println!("best (last max wins) {:?}", best.map(|p| &p.name));

    let mut versions: Vec<Version> = ["1.2.3", "1.10.0", "1.2.3-beta", "0.9.12", "1.2.3-alpha", "2.0.0", "1.2.10"].iter().map(|s| v(s)).collect();
    versions.sort();
    println!("semver: {:?}", versions.iter().map(|v| format!("{}.{}.{}{}", v.major, v.minor, v.patch, v.pre.as_ref().map(|p| format!("-{}", p)).unwrap_or_default())).collect::<Vec<_>>());
    let set: BTreeSet<Version> = versions.into_iter().collect();
    println!("newest {:?}", set.iter().next_back().map(|v| (v.major, v.minor)));

    println!("tuples: {} {} {} {:?}", (1, "b") < (1, "c"), (2, 0) > (1, 99), (1, 2, 3) == (1, 2, 3), (1, 'z').cmp(&(2, 'a')));
    println!("slices: {} {} {} {:?}", [1, 2, 3][..] < [1, 3][..], [1, 2][..] < [1, 2, 0][..], Vec::<i32>::new() < vec![i32::MIN], [3][..].cmp(&[2, 9][..]));
    println!("options: {} {} {:?} results: {}", None < Some(i32::MIN), Some(2) > Some(1), [Some(1), None, Some(0)].iter().min(), Ok::<u8, u8>(9) < Err(0));
    println!("strings: {} {} {} {:?}", "Zebra" < "apple", "apple" < "apples", "é" > "z", "abc".cmp("abd"));
    println!("chars/bools: {} {} {:?}", 'a' < 'b', false < true, [true, false, true].iter().max());
    println!("floats: {:?} {:?} {} {} {:?}", 1.0f64.partial_cmp(&f64::NAN), (-0.0f64).partial_cmp(&0.0), f64::NAN < 1.0, f64::NAN > 1.0, 2.0f64.total_cmp(&-0.0));
    let mut fs = vec![2.5f32, -1.0, 7.25, 0.0];
    fs.sort_by(|a, b| b.partial_cmp(a).unwrap());
    println!("f32 desc {:?} max fold {}", fs, fs.iter().cloned().fold(f32::MIN, f32::max));

    let mut heap: BinaryHeap<(u8, Reverse<&str>)> = BinaryHeap::new();
    for (p, n) in [(2, "b"), (9, "z"), (2, "a"), (9, "c")] {
        heap.push((p, Reverse(n)));
    }
    println!("heap order: {:?}", std::iter::from_fn(|| heap.pop()).map(|(p, Reverse(n))| format!("{}{}", p, n)).collect::<Vec<_>>());
    let words = ["kiwi", "fig", "banana", "apple", "date", "cherry"];
    let mut by_len_then_alpha = words.to_vec();
    by_len_then_alpha.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    println!("len then alpha: {:?}", by_len_then_alpha);
    let mut desc_len = words.to_vec();
    desc_len.sort_unstable_by_key(|w| (Reverse(w.len()), *w));
    println!("desc len (unstable, total key): {:?}", desc_len);
    println!("is_sorted_by: {}", by_len_then_alpha.is_sorted_by(|a, b| a.len() <= b.len()));
    println!("max_by_key last-wins {:?} min_by_key first-wins {:?}", words.iter().max_by_key(|w| w.len()), words.iter().min_by_key(|w| w.len() / 2));
    println!("eq impls: {} {} {}", String::from("x") == "x", "x" == String::from("x"), vec![1] == [1]);
}
