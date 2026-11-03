// std::cmp and range APIs: Ordering combinators, cmp::{min,max}_by(_key),
// clamp, Reverse in tuples, PartialOrd between distinct types, RangeBounds
// generic functions, Bound, RangeInclusive accessors, ranges of chars.
use std::cmp::{self, Ordering, Reverse};
use std::ops::{Bound, RangeBounds, RangeInclusive};

#[derive(Debug, PartialEq, Clone, Copy)]
struct Meters(f64);
#[derive(Debug, PartialEq, Clone, Copy)]
struct Feet(f64);
impl PartialEq<Feet> for Meters { fn eq(&self, o: &Feet) -> bool { (self.0 - o.0 * 0.3048).abs() < 1e-9 } }
impl PartialOrd<Feet> for Meters { fn partial_cmp(&self, o: &Feet) -> Option<Ordering> { self.0.partial_cmp(&(o.0 * 0.3048)) } }
impl PartialOrd for Meters { fn partial_cmp(&self, o: &Meters) -> Option<Ordering> { self.0.partial_cmp(&o.0) } }

#[derive(Debug, Clone, PartialEq, Eq)]
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

fn describe<R: RangeBounds<i32>>(r: R) -> String {
    let s = match r.start_bound() { Bound::Included(x) => format!("[{}", x), Bound::Excluded(x) => format!("({}", x), Bound::Unbounded => "(-inf".into() };
    let e = match r.end_bound() { Bound::Included(x) => format!("{}]", x), Bound::Excluded(x) => format!("{})", x), Bound::Unbounded => "+inf)".into() };
    let probes: Vec<i32> = [-5, 0, 3, 5, 10].into_iter().filter(|p| r.contains(p)).collect();
    format!("{},{} contains {:?}", s, e, probes)
}

fn main() {
    let o = Ordering::Less;
    println!("{:?} {:?} {:?} {} {} {} {}", o.reverse(), o.then(Ordering::Greater), Ordering::Equal.then(Ordering::Greater), o.is_lt(), o.is_ge(), Ordering::Equal.is_eq(), Ordering::Greater.is_gt());
    println!("{:?} {:?} {}", 3.cmp(&5), "b".cmp("abc"), (Ordering::Less as i8));
    println!("min_by_key {:?} max_by {:?}", cmp::min_by_key(-7, 4, |x: &i32| x.abs()), cmp::max_by(2.5, 1.5, |a: &f64, b: &f64| a.partial_cmp(b).unwrap()));
    println!("min/max same keys: {:?} {:?}", cmp::min_by_key((1, 'a'), (1, 'b'), |p| p.0), cmp::max_by_key((1, 'a'), (1, 'b'), |p| p.0));
    println!("clamp {} {} {} {:?}", 15.clamp(0, 10), (-3).clamp(0, 10), 2.5f64.clamp(0.0, 1.0), "m".clamp("a", "k"));
    let mut people = vec![("ann", 30, 5.5), ("bob", 25, 6.1), ("cat", 30, 5.2), ("dan", 25, 5.9)];
    people.sort_by_key(|&(n, age, _)| (Reverse(age), n));
    println!("sort by (Reverse(age), name): {:?}", people.iter().map(|p| p.0).collect::<Vec<_>>());
    people.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    println!("sort by height desc: {:?}", people.iter().map(|p| p.0).collect::<Vec<_>>());
    println!("Reverse ord: {} {:?}", Reverse(1) < Reverse(2), Reverse("a").cmp(&Reverse("b")));

    let m = Meters(3.048);
    println!("cross-type: {} {} {} {:?}", m == Feet(10.0), m < Feet(11.0), m > Feet(9.0), m.partial_cmp(&Feet(10.0)));
    println!("NaN: {:?} {} {}", Meters(f64::NAN).partial_cmp(&Meters(1.0)), Meters(f64::NAN) < Meters(1.0), Meters(f64::NAN) >= Meters(1.0));

    let mut vs: Vec<Version> = ["1.2.3", "1.10.0", "1.2.3-beta", "0.9.12", "1.2.3-alpha", "2.0.0", "1.2.10"].iter().map(|s| v(s)).collect();
    vs.sort();
    println!("{:?}", vs.iter().map(|x| format!("{}.{}.{}{}", x.major, x.minor, x.patch, x.pre.as_ref().map(|p| format!("-{}", p)).unwrap_or_default())).collect::<Vec<_>>());
    println!("max {:?} cmp {:?}", vs.iter().max().map(|x| x.major), v("1.0.0").cmp(&v("1.0.0-rc1")));

    println!("{}", describe(1..5));
    println!("{}", describe(..=3));
    println!("{}", describe(0..));
    println!("{}", describe(..));
    println!("{}", describe((Bound::Excluded(0), Bound::Included(10))));
    println!("{}", describe(5..5));
    let ri: RangeInclusive<u8> = 3..=7;
    println!("start={} end={} len={} sum={} rev={:?} is_empty={}", ri.start(), ri.end(), ri.clone().count(), ri.clone().map(u32::from).sum::<u32>(), ri.clone().rev().collect::<Vec<_>>(), ri.is_empty());
    let (a, b) = ri.into_inner();
    println!("into_inner {} {}; empty inclusive {}", a, b, (5..=4).is_empty());
    println!("chars {:?} count {}", ('a'..='e').collect::<String>(), ('\u{D7FF}'..='\u{E000}').count());
    println!("range contains: {} {} {}", (1.0..2.0).contains(&1.5), (1.0..2.0).contains(&2.0), ('a'..'z').contains(&'z'));
    println!("step_by on range incl: {:?}", (0..=20).step_by(7).collect::<Vec<_>>());
    println!("range of u8 max: {:?}", (250u8..=255).collect::<Vec<_>>());
    println!("rev range i8: {:?}", (-3i8..2).rev().collect::<Vec<_>>());
    let r = 10..20;
    println!("range len {} nth {:?} last {:?}", r.len(), r.clone().nth(3), r.clone().last());
    let sl = [10, 20, 30, 40, 50];
    let b: (Bound<usize>, Bound<usize>) = (Bound::Excluded(1), Bound::Unbounded);
    println!("slice by bounds {:?} get range {:?} {:?}", &sl[b], sl.get(1..3), sl.get(4..9));
}
