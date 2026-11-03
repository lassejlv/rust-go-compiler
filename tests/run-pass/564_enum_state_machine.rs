// state machines with enums: traffic light, vending machine with transitions consuming self, parser FSM
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Light {
    Red { secs: u8 },
    Green { secs: u8 },
    Yellow { secs: u8 },
    Flashing,
}

impl Light {
    fn tick(self) -> Light {
        match self {
            Light::Red { secs: 0 } => Light::Green { secs: 3 },
            Light::Green { secs: 0 } => Light::Yellow { secs: 1 },
            Light::Yellow { secs: 0 } => Light::Red { secs: 2 },
            Light::Red { secs } => Light::Red { secs: secs - 1 },
            Light::Green { secs } => Light::Green { secs: secs - 1 },
            Light::Yellow { secs } => Light::Yellow { secs: secs - 1 },
            Light::Flashing => Light::Flashing,
        }
    }
}

#[derive(Debug)]
enum Vending {
    Idle,
    HasMoney(u32),
    Dispensing { item: String, change: u32 },
    OutOfOrder(String),
}

#[derive(Debug)]
enum Event {
    Coin(u32),
    Select(&'static str, u32),
    Take,
    Kick,
}

impl Vending {
    fn on(self, e: &Event) -> Vending {
        use Vending::*;
        match (self, e) {
            (OutOfOrder(r), _) => OutOfOrder(r),
            (_, Event::Kick) => OutOfOrder("kicked".to_string()),
            (Idle, Event::Coin(c)) => HasMoney(*c),
            (HasMoney(m), Event::Coin(c)) => HasMoney(m + c),
            (HasMoney(m), Event::Select(item, price)) if m >= *price => Dispensing { item: item.to_string(), change: m - price },
            (HasMoney(m), Event::Select(..)) => HasMoney(m),
            (Dispensing { .. }, Event::Take) => Idle,
            (s @ Dispensing { .. }, _) => s,
            (s, _) => s,
        }
    }
}

impl fmt::Display for Vending {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Vending::Idle => write!(f, "idle"),
            Vending::HasMoney(m) => write!(f, "credit {}", m),
            Vending::Dispensing { item, change } => write!(f, "dispensing {} (change {})", item, change),
            Vending::OutOfOrder(r) => write!(f, "out of order: {}", r),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NumState {
    Start,
    Sign,
    Int,
    Dot,
    Frac,
    Exp,
    ExpSign,
    ExpInt,
}

fn is_number(s: &str) -> bool {
    use NumState::*;
    let mut st = Start;
    let mut seen_digit = false;
    for c in s.chars() {
        st = match (st, c) {
            (Start, '+' | '-') => Sign,
            (Start | Sign | Int, '0'..='9') => {
                seen_digit = true;
                Int
            }
            (Start | Sign | Int, '.') => Dot,
            (Dot | Frac, '0'..='9') => {
                seen_digit = true;
                Frac
            }
            (Int | Frac, 'e' | 'E') => Exp,
            (Dot, 'e' | 'E') if seen_digit => Exp,
            (Exp, '+' | '-') => ExpSign,
            (Exp | ExpSign | ExpInt, '0'..='9') => ExpInt,
            _ => return false,
        };
    }
    matches!(st, Int | Frac | ExpInt) || (st == Dot && seen_digit)
}

fn main() {
    let mut l = Light::Red { secs: 1 };
    let mut trace = vec![];
    for _ in 0..10 {
        trace.push(format!("{:?}", l));
        l = l.tick();
    }
    println!("{}", trace.join(" -> "));
    println!("{:?}", Light::Flashing.tick());
    let events = [
        Event::Select("cola", 150),
        Event::Coin(100),
        Event::Select("cola", 150),
        Event::Coin(25),
        Event::Coin(50),
        Event::Select("cola", 150),
        Event::Coin(5),
        Event::Take,
        Event::Coin(10),
        Event::Kick,
        Event::Coin(100),
    ];
    let mut m = Vending::Idle;
    for e in &events {
        m = m.on(e);
        println!("{:<20} => {}", format!("{:?}", e), m);
    }
    for s in ["0", "-12", "+3.5", ".5", "5.", "1e10", "1.5E-3", "e9", ".", "-.e1", "1e", "--1", "12a", "3.e2", ""] {
        print!("{:?}={} ", s, is_number(s));
    }
    println!();
}
