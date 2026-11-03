// Discrete-event simulation of a small bank of elevators: a time-ordered
// event queue (BinaryHeap<Reverse<(time, seq, Event)>>), Rc<RefCell<>>
// shared state, observer callbacks registered as Box<dyn FnMut>, Weak
// observers that unregister when dropped, and summary statistics.
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::rc::{Rc, Weak};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Event { Call { floor: u8, dest: u8, id: u32 }, Arrive { car: usize, floor: u8 }, Report }

#[derive(Debug, Default)]
struct Car { floor: u8, queue: Vec<(u8, u32, bool)>, moving: bool, served: u32, travel: u32 } // (floor, passenger, is_pickup)

#[derive(Default)]
struct Stats { waits: BTreeMap<u32, u64>, rides: BTreeMap<u32, u64> }

struct Sim {
    now: u64,
    seq: u64,
    q: BinaryHeap<Reverse<(u64, u64, Event)>>,
    cars: Vec<Car>,
    called_at: BTreeMap<u32, u64>,
    picked_at: BTreeMap<u32, u64>,
    observers: Vec<Box<dyn FnMut(u64, &str)>>,
    weak_obs: Vec<Weak<RefCell<Vec<String>>>>,
    stats: Rc<RefCell<Stats>>,
}

impl Sim {
    fn schedule(&mut self, dt: u64, e: Event) { self.seq += 1; self.q.push(Reverse((self.now + dt, self.seq, e))); }
    fn notify(&mut self, msg: String) {
        let now = self.now;
        for o in self.observers.iter_mut() { o(now, &msg); }
        self.weak_obs.retain(|w| match w.upgrade() { Some(log) => { log.borrow_mut().push(format!("t{} {}", now, msg)); true } None => false });
    }
    fn pick_car(&self, floor: u8) -> usize {
        (0..self.cars.len()).min_by_key(|&i| (self.cars[i].queue.len(), (self.cars[i].floor as i32 - floor as i32).abs(), i)).unwrap()
    }
    fn dispatch(&mut self, car: usize) {
        let c = &mut self.cars[car];
        if c.moving { return; }
        if let Some(&(target, _, _)) = c.queue.first() {
            let dist = (target as i32 - c.floor as i32).unsigned_abs();
            c.moving = true;
            c.travel += dist;
            self.schedule(dist as u64 * 2 + 1, Event::Arrive { car, floor: target });
        }
    }
    fn step(&mut self) -> bool {
        let Some(Reverse((t, _, e))) = self.q.pop() else { return false };
        self.now = t;
        match e {
            Event::Call { floor, dest, id } => {
                let car = self.pick_car(floor);
                self.called_at.insert(id, t);
                self.cars[car].queue.push((floor, id, true));
                self.notify(format!("p{} calls at {} -> {} (car {})", id, floor, dest, car));
                self.cars[car].queue.push((dest, id, false));
                self.dispatch(car);
            }
            Event::Arrive { car, floor } => {
                let c = &mut self.cars[car];
                c.floor = floor;
                c.moving = false;
                let picked = &self.picked_at;
                let (here, rest): (Vec<_>, Vec<_>) = c.queue.drain(..).partition(|(f, id, pickup)| *f == floor && (*pickup || picked.contains_key(id)));
                c.queue = rest;
                let mut msgs = vec![];
                for (_, id, pickup) in here {
                    if pickup {
                        let w = t - self.called_at[&id];
                        self.picked_at.insert(id, t);
                        self.stats.borrow_mut().waits.insert(id, w);
                        msgs.push(format!("car {} picks p{} at {} (waited {})", car, id, floor, w));
                    } else {
                        let r = t - self.picked_at[&id];
                        self.cars[car].served += 1;
                        self.stats.borrow_mut().rides.insert(id, r);
                        msgs.push(format!("car {} drops p{} at {} (ride {})", car, id, floor, r));
                    }
                }
                for m in msgs { self.notify(m); }
                self.dispatch(car);
            }
            Event::Report => {
                let s = format!("report: floors {:?}", self.cars.iter().map(|c| c.floor).collect::<Vec<_>>());
                self.notify(s);
            }
        }
        true
    }
}

fn main() {
    let stats = Rc::new(RefCell::new(Stats::default()));
    let mut sim = Sim { now: 0, seq: 0, q: BinaryHeap::new(), cars: (0..2).map(|_| Car::default()).collect(), called_at: BTreeMap::new(), picked_at: BTreeMap::new(), observers: vec![], weak_obs: vec![], stats: stats.clone() };
    let count = Rc::new(RefCell::new(0u32));
    let c2 = count.clone();
    sim.observers.push(Box::new(move |_, _| *c2.borrow_mut() += 1));
    sim.observers.push(Box::new(|t, m| println!("[{:>3}] {}", t, m)));
    let early_log = Rc::new(RefCell::new(Vec::new()));
    sim.weak_obs.push(Rc::downgrade(&early_log));
    let calls = [(0, 0, 5, 1), (2, 3, 0, 2), (3, 7, 2, 3), (4, 1, 8, 4), (10, 9, 0, 5), (11, 4, 4, 6), (12, 2, 6, 7)];
    for (t, floor, dest, id) in calls { sim.now = 0; sim.schedule(t, Event::Call { floor, dest, id }); }
    sim.now = 0;
    sim.schedule(15, Event::Report);
    let mut steps = 0;
    while sim.step() {
        steps += 1;
        if steps == 6 {
            let n = early_log.borrow().len();
            println!("-- dropping early observer after {} messages --", n);
            drop(std::mem::take(&mut *early_log.borrow_mut()));
        }
        if steps == 7 { break; }
    }
    drop(early_log);
    while sim.step() { steps += 1; }
    let st = stats.borrow();
    let avg = |m: &BTreeMap<u32, u64>| m.values().sum::<u64>() as f64 / m.len().max(1) as f64;
    println!("steps {} notifications {} weak observers left {}", steps, count.borrow(), sim.weak_obs.len());
    println!("waits {:?} avg {:.2}", st.waits, avg(&st.waits));
    println!("rides {:?} avg {:.2}", st.rides, avg(&st.rides));
    for (i, c) in sim.cars.iter().enumerate() { println!("car {}: floor {} served {} travel {}", i, c.floor, c.served, c.travel); }
}
