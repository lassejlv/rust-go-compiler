struct Noisy {
    name: String,
}

impl Noisy {
    fn new(name: &str) -> Noisy {
        println!("create {}", name);
        Noisy { name: name.to_string() }
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

struct Pair {
    first: Noisy,
    second: Noisy,
}

impl Drop for Pair {
    fn drop(&mut self) {
        println!("drop Pair({}, {})", self.first.name, self.second.name);
    }
}

fn consume(n: Noisy) {
    println!("consuming {}", n.name);
}

fn make_and_return() -> Noisy {
    let _temp = Noisy::new("temp-in-fn");
    let keep = Noisy::new("returned");
    keep
}

struct Guard<'a> {
    label: &'a str,
    depth: usize,
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        println!("{}exit {}", "  ".repeat(self.depth), self.label);
    }
}

fn nested(depth: usize) {
    println!("{}enter level{}", "  ".repeat(depth), depth);
    let _g = Guard { label: "level", depth };
    if depth < 2 {
        nested(depth + 1);
    }
}

fn main() {
    let _a = Noisy::new("a");
    {
        let _b = Noisy::new("b");
        let _c = Noisy::new("c");
        println!("end of inner scope");
    }
    let d = Noisy::new("d");
    consume(d);
    println!("after consume");
    let e = Noisy::new("e");
    drop(e);
    println!("after explicit drop");
    let r = make_and_return();
    println!("got {}", r.name);
    let _p = Pair { first: Noisy::new("p1"), second: Noisy::new("p2") };
    let mut v = vec![Noisy::new("v1"), Noisy::new("v2"), Noisy::new("v3")];
    let popped = v.remove(1);
    println!("removed {}", popped.name);
    drop(v);
    println!("vec dropped");
    let mut slot = Some(Noisy::new("opt"));
    slot = None;
    println!("slot is_none = {}", slot.is_none());
    let mut reassigned = Noisy::new("old");
    reassigned = Noisy::new("new");
    println!("reassigned to {}", reassigned.name);
    let boxed: Box<Noisy> = Box::new(Noisy::new("boxed"));
    let moved_box = boxed;
    println!("box moved: {}", moved_box.name);
    let _ = Noisy::new("underscore");
    println!("after let _");
    nested(0);
    let shadow = Noisy::new("shadow1");
    let shadow = Noisy { name: format!("{}-2", shadow.name) };
    println!("shadowed: {}", shadow.name);
    println!("end of main");
}
