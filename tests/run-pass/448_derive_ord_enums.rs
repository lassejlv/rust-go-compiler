// Derived PartialOrd/Ord/PartialEq/Hash on enums and structs: variant declaration order, field-wise
// lexicographic ordering, enums with explicit discriminants (ordering follows declaration, not value),
// f64 fields with partial ordering, Option/Result ordering, tuples, sort/min/max/BTreeSet with derives.
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Level {
    Low = 10,
    High = 1,
    Mid = 5,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Event {
    Start,
    Data(u32, String),
    Tagged { level: Level, id: i8 },
    End,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct Reading {
    sensor: &'static str,
    value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Version {
    major: u16,
    minor: u16,
    patch: u16,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Wrapper(Option<Version>, Reverse2);

#[derive(Debug, PartialEq, Eq)]
struct Reverse2(i32);
impl PartialOrd for Reverse2 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Reverse2 {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.cmp(&self.0)
    }
}

fn v(major: u16, minor: u16, patch: u16) -> Version {
    Version { major, minor, patch }
}

fn main() {
    let mut levels = vec![Level::Mid, Level::Low, Level::High, Level::Mid];
    levels.sort();
    println!("{:?}", levels);
    println!("{:?}", levels.iter().map(|l| *l as i32).collect::<Vec<_>>());
    println!("{} {}", Level::Low < Level::High, (Level::Low as i32) < (Level::High as i32));
    println!("{:?} {:?}", Level::High.cmp(&Level::Mid), Level::Mid.max(Level::Low));

    let mut events = vec![
        Event::End,
        Event::Data(2, "b".into()),
        Event::Tagged { level: Level::Mid, id: -1 },
        Event::Data(2, "a".into()),
        Event::Start,
        Event::Tagged { level: Level::Low, id: 5 },
        Event::Data(1, "z".into()),
        Event::Tagged { level: Level::Mid, id: -3 },
    ];
    events.sort();
    for e in &events {
        println!("  {:?}", e);
    }
    let set: BTreeSet<Event> = events.iter().cloned().collect();
    println!("first={:?} last={:?}", set.first(), set.last());
    let hs: HashSet<Event> = events.into_iter().collect();
    println!("hash set len {}", hs.len());

    let mut rs = vec![
        Reading { sensor: "b", value: 2.0 },
        Reading { sensor: "a", value: f64::NAN },
        Reading { sensor: "a", value: 1.0 },
    ];
    println!("{:?}", rs[1].partial_cmp(&rs[2]));
    println!("{:?}", rs[0].partial_cmp(&rs[2]));
    println!("{}", rs[1] == rs[1].clone());
    rs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    println!("{:?}", rs.iter().map(|r| r.sensor).collect::<Vec<_>>());

    let mut versions = vec![v(1, 2, 3), v(1, 10, 0), v(0, 99, 99), v(1, 2, 10), v(2, 0, 0)];
    versions.sort();
    println!("{:?}", versions.iter().map(|x| format!("{}.{}.{}", x.major, x.minor, x.patch)).collect::<Vec<_>>());
    println!("max={:?}", versions.iter().max());
    println!("{:?}", v(1, 2, 3).cmp(&v(1, 2, 3)));
    println!("{}", v(1, 2, 3).clamp(v(1, 5, 0), v(3, 0, 0)) == v(1, 5, 0));

    let mut ws = vec![
        Wrapper(Some(v(1, 0, 0)), Reverse2(1)),
        Wrapper(None, Reverse2(5)),
        Wrapper(Some(v(1, 0, 0)), Reverse2(9)),
        Wrapper(None, Reverse2(-5)),
    ];
    ws.sort();
    println!("{:?}", ws.iter().map(|w| (w.0.is_some(), w.1 .0)).collect::<Vec<_>>());

    let opts = [Some(3), None, Some(-1)];
    let mut o = opts.to_vec();
    o.sort();
    println!("{:?}", o);
    let mut res: Vec<Result<i32, i32>> = vec![Err(1), Ok(5), Err(-2), Ok(0)];
    res.sort();
    println!("{:?}", res);
    let mut tup = vec![(2, 'b', "x"), (1, 'z', "y"), (2, 'a', "z"), (2, 'b', "a")];
    tup.sort();
    println!("{:?}", tup);
    println!("{:?} {:?}", (1, 2.0).partial_cmp(&(1, f64::NAN)), [1, 2, 3][..].cmp(&[1, 2][..]));
    println!("{:?} {:?}", "abc".cmp("abd"), "Z".cmp("a"));
    println!("{:?}", vec![3, 1].cmp(&vec![2, 9, 9]));
    let mut strs = vec!["b10", "b9", "a", "B1", ""];
    strs.sort();
    println!("{:?}", strs);
    strs.sort_by_key(|s| (s.len(), s.to_lowercase()));
    println!("{:?}", strs);
    println!("{:?}", Ordering::Less.then(Ordering::Greater).reverse());
    println!("{:?}", 1.cmp(&1).then_with(|| 'b'.cmp(&'a')));
}
