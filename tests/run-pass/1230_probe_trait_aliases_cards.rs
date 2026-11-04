mod a01 {
    use std::fmt::Display;
    use std::ops::Add;
    trait Num: Add<Output = Self> + Copy + Default + Display {}
    impl<T: Add<Output = T> + Copy + Default + Display> Num for T {}
    fn total<T: Num>(xs: &[T]) -> T { xs.iter().fold(T::default(), |a, &b| a + b) }
    fn show_items<I>(it: I) -> String where I: Iterator<Item: Display> { it.map(|x| x.to_string()).collect::<Vec<_>>().join("|") }
    fn adder(a: i32) -> impl Fn(i32) -> Box<dyn Fn(i32) -> i32> { move |b| Box::new(move |c| a + b + c) }
    trait Area { fn area(&self) -> f64; }
    impl Area for [f64; 2] { fn area(&self) -> f64 { self[0] * self[1] } }
    impl<T: Area> Area for Vec<T> { fn area(&self) -> f64 { self.iter().map(Area::area).sum() } }
    const INC: fn(i32) -> i32 = |x| x + 1;
    static mut COUNTER: u32 = 0;
    fn bump() -> u32 { unsafe { COUNTER += 1; COUNTER } }
    struct Sz<T>(std::marker::PhantomData<T>);
    impl<T> Sz<T> { const SIZE: usize = std::mem::size_of::<T>(); const DOUBLE: usize = Self::SIZE * 2; }
    #[derive(Debug, PartialEq)]
    enum Shape<T> { Circle(T), Square(T), Nothing }
    impl Shape<f64> { fn describe(&self) -> String { format!("float shape {:?}", self) } }
    impl<T: Copy + Into<f64>> Shape<T> {
        fn size(&self) -> f64 { use Shape::*; match *self { Circle(r) => 3.0 * r.into() * r.into(), Square(s) => s.into() * s.into(), Nothing => 0.0 } }
        fn is(&self, other: &Self) -> bool where T: PartialEq { self == other }
    }
    pub fn main() {
        println!("{} {}", total(&[1, 2, 3]), total(&[0.5, 0.25]));
        println!("{}", show_items(vec![1.5, 2.0].into_iter()));
        println!("{}", adder(1)(2)(3));
        println!("{}", vec![[1.0, 2.0], [3.0, 0.5]].area());
        println!("{}", INC(INC(1)));
        bump(); bump();
        println!("{}", bump());
        println!("{} {} {}", Sz::<u64>::SIZE, Sz::<(u8, u32)>::DOUBLE, Sz::<()>::SIZE);
        println!("{} {}", Shape::Circle(1.0f64).describe(), Shape::Square(3u8).size());
        println!("{} {}", Shape::Circle(2i32).size(), Shape::<u8>::Nothing.is(&Shape::Nothing));
        println!("{} {} {}", -5i32.abs(), (-5i32).abs(), 2i32.pow(10));
        println!("{} {}", -2.0f64.powi(2), (-2.0f64).powi(2));
        let (a, b): (u8, _) = (1, "x");
        println!("{} {}", a, b);
        let v = Some(5);
        let msg = if let Some(x @ 0..=3) = v { format!("small {}", x) } else if let Some(x) = v { format!("big {}", x) } else { "none".into() };
        println!("{}", msg);
        let data = vec![("a", 3), ("b", 1)];
        let Some(&(name, _)) = data.iter().min_by_key(|(_, n)| *n) else { panic!() };
        println!("{}", name);
        let x = 5;
        let r = match x { x if x > 3 => { let x = x * 2; x + 1 } x => x };
        println!("{}", r);
    }
}

mod a02 {
    use std::collections::HashMap;
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    enum Suit { Clubs, Diamonds, Hearts, Spades }
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    struct Card { rank: u8, suit: Suit }
    impl std::fmt::Display for Card {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            let r = match self.rank { 11 => "J".to_string(), 12 => "Q".into(), 13 => "K".into(), 14 => "A".into(), n => n.to_string() };
            let s = match self.suit { Suit::Clubs => '♣', Suit::Diamonds => '♦', Suit::Hearts => '♥', Suit::Spades => '♠' };
            write!(f, "{}{}", r, s)
        }
    }
    fn parse(s: &str) -> Vec<Card> {
        s.split_whitespace().map(|t| {
            let (r, su) = t.split_at(t.len() - 1);
            let rank = match r { "J" => 11, "Q" => 12, "K" => 13, "A" => 14, n => n.parse().unwrap() };
            let suit = match su { "c" => Suit::Clubs, "d" => Suit::Diamonds, "h" => Suit::Hearts, _ => Suit::Spades };
            Card { rank, suit }
        }).collect()
    }
    fn classify(hand: &[Card]) -> (u8, Vec<u8>) {
        let mut counts: HashMap<u8, u8> = HashMap::new();
        for c in hand { *counts.entry(c.rank).or_default() += 1; }
        let mut groups: Vec<(u8, u8)> = counts.into_iter().map(|(r, n)| (n, r)).collect();
        groups.sort_by(|a, b| b.cmp(a));
        let flush = hand.windows(2).all(|w| w[0].suit == w[1].suit);
        let mut ranks: Vec<u8> = hand.iter().map(|c| c.rank).collect();
        ranks.sort_unstable_by(|a, b| b.cmp(a));
        let straight = ranks.windows(2).all(|w| w[0] == w[1] + 1);
        let cat = match (groups[0].0, groups.get(1).map(|g| g.0), flush, straight) {
            (_, _, true, true) => 8,
            (4, _, _, _) => 7,
            (3, Some(2), _, _) => 6,
            (_, _, true, _) => 5,
            (_, _, _, true) => 4,
            (3, _, _, _) => 3,
            (2, Some(2), _, _) => 2,
            (2, _, _, _) => 1,
            _ => 0,
        };
        (cat, groups.iter().map(|g| g.1).collect())
    }
    pub fn main() {
        let hands = ["2h 3h 4h 5h 6h", "Kd Kc Ks Kh 2c", "3c 3d 3s 9h 9d", "Ah 9h 4h 2h Jh", "5c 6d 7s 8h 9c", "Qc Qd Qs 2h 4d", "Jc Jd 4s 4h Ac", "Tc 2d 2s 5h 6c", "Ac Kd 9s 7h 3c"];
        let mut scored: Vec<(String, (u8, Vec<u8>))> = hands.iter().map(|h| {
            let h = h.replace('T', "10");
            let cards = parse(&h);
            let shown: Vec<String> = cards.iter().map(|c| c.to_string()).collect();
            (shown.join(" "), classify(&cards))
        }).collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        for (h, (c, k)) in &scored { println!("{:<22} cat={} kick={:?}", h, c, k); }
        let mut deck: Vec<Card> = [Suit::Spades, Suit::Hearts].iter().flat_map(|&s| (13..=14).map(move |r| Card { rank: r, suit: s })).collect();
        deck.sort();
        println!("{}", deck.iter().map(|c| format!("{:>4}", c.to_string())).collect::<String>());
        println!("{:?}", deck.iter().max());
    }
}

fn main() {
    println!("== a01");
    a01::main();
    println!("== a02");
    a02::main();
}
