use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

trait Machine {
    type State: Copy + Eq + Hash + Debug;
    type Event: Debug;
    type Output: Debug;
    const START: Self::State;
    fn step(&mut self, s: Self::State, e: &Self::Event) -> Option<(Self::State, Option<Self::Output>)>;
    fn is_final(s: Self::State) -> bool;
}

fn run<M: Machine, I>(m: &mut M, events: I) -> (M::State, Vec<M::Output>, HashMap<M::State, usize>)
where I: IntoIterator<Item = M::Event> {
    let mut s = M::START;
    let mut out = vec![];
    let mut visits = HashMap::new();
    *visits.entry(s).or_insert(0) += 1;
    for e in events {
        if M::is_final(s) { break; }
        match m.step(s, &e) {
            Some((ns, o)) => { s = ns; out.extend(o); *visits.entry(s).or_insert(0) += 1; }
            None => { println!("  ignored {:?} in {:?}", e, s); }
        }
    }
    (s, out, visits)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
enum Door { Closed, Open, Locked, Broken }
#[derive(Debug)]
enum DoorEv { Push, Pull, Lock(u32), Unlock(u32), Kick }
struct DoorM { code: u32, kicks: u8 }
impl Machine for DoorM {
    type State = Door;
    type Event = DoorEv;
    type Output = String;
    const START: Door = Door::Closed;
    fn step(&mut self, s: Door, e: &DoorEv) -> Option<(Door, Option<String>)> {
        use Door::*;
        Some(match (s, e) {
            (Closed, DoorEv::Pull) => (Open, Some("creak".into())),
            (Open, DoorEv::Push) => (Closed, None),
            (Closed, DoorEv::Lock(c)) => { self.code = *c; (Locked, Some(format!("locked {}", c))) }
            (Locked, DoorEv::Unlock(c)) if *c == self.code => (Closed, Some("click".into())),
            (_, DoorEv::Kick) => { self.kicks += 1; if self.kicks >= 2 { (Broken, Some("crash".into())) } else { (s, Some("thud".into())) } }
            _ => return None,
        })
    }
    fn is_final(s: Door) -> bool { s == Door::Broken }
}

struct Parity;
impl Machine for Parity {
    type State = bool;
    type Event = u8;
    type Output = u8;
    const START: bool = false;
    fn step(&mut self, s: bool, e: &u8) -> Option<(bool, Option<u8>)> { Some((s ^ (e.count_ones() % 2 == 1), if *e > 100 { Some(*e) } else { None })) }
    fn is_final(_: bool) -> bool { false }
}

fn sorted<K: Ord + Copy, V: Copy>(m: &HashMap<K, V>) -> Vec<(K, V)> { let mut v: Vec<_> = m.iter().map(|(k, v)| (*k, *v)).collect(); v.sort_by_key(|p| p.0); v }

fn main() {
    let mut d = DoorM { code: 0, kicks: 0 };
    let (s, out, visits) = run(&mut d, vec![DoorEv::Pull, DoorEv::Lock(1), DoorEv::Push, DoorEv::Lock(42), DoorEv::Unlock(7), DoorEv::Unlock(42), DoorEv::Kick, DoorEv::Kick, DoorEv::Pull]);
    println!("{:?} {:?} {:?}", s, out, sorted(&visits));
    let (p, big, pv) = run(&mut Parity, [1u8, 3, 200, 7, 255]);
    println!("{} {:?} {:?}", p, big, sorted(&pv));
}
