// edition:2018
// Edition 2018 closures capture whole variables (no disjoint field capture),
// which is observable through Drop order, through what remains usable after
// a `move` closure, and through which fields get mutably borrowed.
use std::cell::RefCell;

thread_local! { static EVENTS: RefCell<Vec<String>> = RefCell::new(Vec::new()); }

fn ev(s: impl Into<String>) { EVENTS.with(|e| e.borrow_mut().push(s.into())); }

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) { ev(format!("drop {}", self.0)); }
}

struct Holder { first: Noisy, second: Noisy, count: u32 }

fn scenario_move_one_field() {
    ev("-- move closure touching only h.first");
    let h = Holder { first: Noisy("h.first"), second: Noisy("h.second"), count: 0 };
    let c = move || { let _ = &h.first; ev("closure body"); };
    c();
    drop(c); // in 2018 this drops ALL of h; in 2021 only h.first
    ev("after drop(c)");
}

fn scenario_tuple() {
    ev("-- tuple field capture");
    let t = (Noisy("t.0"), Noisy("t.1"));
    {
        let c = move || { let _x = &t.1; };
        c();
        ev("closure going out of scope");
    }
    ev("after block");
}

fn scenario_mut_borrow() -> u32 {
    let mut h = Holder { first: Noisy("m.first"), second: Noisy("m.second"), count: 0 };
    {
        let mut bump = || h.count += 1; // borrows all of `h` mutably in 2018
        bump();
        bump();
    }
    let _ = &h.second; // fine once the closure is gone
    h.count
}

fn scenario_ref_closure() {
    ev("-- non-move closure using a field by value");
    let h = Holder { first: Noisy("r.first"), second: Noisy("r.second"), count: 5 };
    let c = || {
        let f = h.first; // moves h.first => in 2018 the closure captures `h` by value
        ev(format!("closure moved {} (count {})", f.0, h.count));
    };
    ev("closure created");
    c();
    ev("closure called");
}

fn main() {
    scenario_move_one_field();
    scenario_tuple();
    ev(format!("count = {}", scenario_mut_borrow()));
    scenario_ref_closure();
    EVENTS.with(|e| {
        for (i, line) in e.borrow().iter().enumerate() {
            println!("{:>2}: {}", i, line);
        }
    });
}
