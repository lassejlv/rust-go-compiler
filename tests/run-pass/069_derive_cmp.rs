#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Card {
    rank: u8,
    suit: Suit,
}

#[derive(Debug, PartialEq, PartialOrd)]
struct Version(u32, u32, u32);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Low,
    Medium(u8),
    High { urgent: bool },
}

fn main() {
    println!("Clubs < Spades: {}", Suit::Clubs < Suit::Spades);
    println!("Hearts == Hearts: {}", Suit::Hearts == Suit::Hearts);
    println!("max suit: {:?}", Suit::Diamonds.max(Suit::Hearts));
    let mut hand = vec![
        Card { rank: 10, suit: Suit::Hearts },
        Card { rank: 2, suit: Suit::Spades },
        Card { rank: 10, suit: Suit::Clubs },
        Card { rank: 14, suit: Suit::Diamonds },
        Card { rank: 2, suit: Suit::Clubs },
    ];
    hand.sort();
    for c in &hand {
        println!("  {:?}", c);
    }
    println!("max card: {:?}", hand.iter().max().unwrap());
    println!("min card: {:?}", hand.iter().min().unwrap());
    let a = Card { rank: 5, suit: Suit::Hearts };
    let b = Card { rank: 5, suit: Suit::Spades };
    println!("a < b: {}, a == a: {}, a != b: {}", a < b, a == a, a != b);
    println!("cmp: {:?} {:?} {:?}", a.cmp(&b), b.cmp(&a), a.cmp(&a));
    let v1 = Version(1, 2, 3);
    let v2 = Version(1, 10, 0);
    println!("{:?} < {:?}: {}", v1, v2, v1 < v2);
    println!("partial_cmp: {:?}", v1.partial_cmp(&v2));
    let mut ps = vec![
        Priority::High { urgent: false },
        Priority::Low,
        Priority::Medium(5),
        Priority::High { urgent: true },
        Priority::Medium(1),
    ];
    ps.sort();
    println!("priorities: {:?}", ps);
    println!("Medium(9) < High: {}", Priority::Medium(9) < Priority::High { urgent: false });
    let tuples = vec![(2, "b"), (1, "z"), (2, "a")];
    let mut t2 = tuples.clone();
    t2.sort();
    println!("tuples sorted: {:?}", t2);
    println!("strings: {}", "apple".cmp("apricot") == std::cmp::Ordering::Less);
    println!("clamp: {}", 15.clamp(0, 10));
    println!("vec cmp: {}", vec![1, 2, 3] < vec![1, 3]);
}
