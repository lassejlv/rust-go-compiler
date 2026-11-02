#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Selecting { item: u8 },
    Paying { item: u8, paid: u32 },
    Dispensing { item: u8, change: u32 },
    OutOfOrder,
}

#[derive(Debug, Clone, Copy)]
enum Event {
    Select(u8),
    Coin(u32),
    Cancel,
    Done,
    Kick,
    Repair,
}

fn price(item: u8) -> Option<u32> {
    match item {
        1 => Some(125),
        2 => Some(200),
        3 => Some(75),
        _ => None,
    }
}

struct Machine {
    state: State,
    bank: u32,
    log: Vec<String>,
}

impl Machine {
    fn new() -> Self {
        Machine { state: State::Idle, bank: 0, log: vec![] }
    }

    fn handle(&mut self, ev: Event) {
        use State::*;
        let next = match (self.state, ev) {
            (OutOfOrder, Event::Repair) => Idle,
            (OutOfOrder, _) => OutOfOrder,
            (_, Event::Kick) => {
                self.log.push("machine kicked!".to_string());
                OutOfOrder
            }
            (Idle, Event::Select(i)) | (Selecting { .. }, Event::Select(i)) => match price(i) {
                Some(p) => {
                    self.log.push(format!("selected {} costing {}", i, p));
                    Selecting { item: i }
                }
                None => {
                    self.log.push(format!("invalid item {}", i));
                    Idle
                }
            },
            (Selecting { item }, Event::Coin(c)) | (Paying { item, paid: 0 }, Event::Coin(c)) => {
                self.check_paid(item, c)
            }
            (Paying { item, paid }, Event::Coin(c)) => self.check_paid(item, paid + c),
            (Paying { paid, .. }, Event::Cancel) => {
                self.log.push(format!("refund {}", paid));
                Idle
            }
            (Selecting { .. }, Event::Cancel) => Idle,
            (Dispensing { item, change }, Event::Done) => {
                self.bank += price(item).unwrap();
                self.log.push(format!("dispensed {} with change {}", item, change));
                Idle
            }
            (s, e) => {
                self.log.push(format!("ignored {:?} in {:?}", e, s));
                s
            }
        };
        println!("{:<10} {:?} -> {:?}", format!("{:?}", ev), self.state, next);
        self.state = next;
    }

    fn check_paid(&mut self, item: u8, paid: u32) -> State {
        let p = price(item).unwrap();
        if paid >= p {
            State::Dispensing { item, change: paid - p }
        } else {
            State::Paying { item, paid }
        }
    }
}

fn main() {
    let mut m = Machine::new();
    let events = [
        Event::Coin(25),
        Event::Select(9),
        Event::Select(1),
        Event::Coin(100),
        Event::Coin(50),
        Event::Done,
        Event::Select(3),
        Event::Select(2),
        Event::Coin(100),
        Event::Cancel,
        Event::Select(3),
        Event::Coin(75),
        Event::Done,
        Event::Kick,
        Event::Select(1),
        Event::Repair,
        Event::Done,
    ];
    for e in events.iter() {
        m.handle(*e);
    }
    println!("--- log ---");
    for (i, l) in m.log.iter().enumerate() {
        println!("{:2}: {}", i, l);
    }
    println!("bank = {}", m.bank);
    println!("final state = {:?}, idle? {}", m.state, m.state == State::Idle);
}
