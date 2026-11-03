// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p05_match_mut_enum_split {
    #[derive(Debug)]
    enum Shape { Rect { w: i32, h: i32 }, Pair(String, String), Empty }
    fn tweak(s: &mut Shape) {
        match s {
            Shape::Rect { w, h } => { std::mem::swap(w, h); *w += *h; }
            Shape::Pair(a, b) => { a.push_str(b); b.clear(); }
            Shape::Empty => {}
        }
    }
    pub fn main() {
        let mut shapes = vec![Shape::Rect { w: 1, h: 2 }, Shape::Pair("x".into(), "y".into()), Shape::Empty];
        for s in shapes.iter_mut() { tweak(s); }
        if let Some(Shape::Rect { w, .. }) = shapes.first_mut() { *w *= 100; }
        match &mut shapes[1] {
            Shape::Pair(a, _) => a.push('!'),
            _ => unreachable!(),
        }
        println!("{:?}", shapes);
    }
}

mod p47_enum_methods_mut_self {
    #[derive(Debug)]
    enum State { Idle, Running { ticks: u32, log: Vec<String> }, Done(String) }
    impl State {
        fn tick(&mut self) {
            match self {
                State::Idle => *self = State::Running { ticks: 0, log: vec![] },
                State::Running { ticks, log } if *ticks >= 2 => {
                    let summary = format!("{} ticks, {} logs", ticks, log.len());
                    *self = State::Done(summary);
                }
                State::Running { ticks, log } => { *ticks += 1; log.push(format!("t{}", ticks)); }
                State::Done(_) => {}
            }
        }
        fn log_mut(&mut self) -> Option<&mut Vec<String>> {
            if let State::Running { log, .. } = self { Some(log) } else { None }
        }
        fn take_done(&mut self) -> Option<String> {
            match std::mem::replace(self, State::Idle) {
                State::Done(s) => Some(s),
                other => { *self = other; None }
            }
        }
    }
    pub fn main() {
        let mut s = State::Idle;
        let mut seen = vec![];
        for _ in 0..5 {
            s.tick();
            if let Some(l) = s.log_mut() { l.push("x".into()); seen.push(l.len()); }
        }
        let d = s.take_done();
        println!("{:?} {:?} {:?}", seen, d, s);
    }
}

mod p56_match_on_tuple_refs_and_bindings {
    #[derive(Debug, Clone, PartialEq)]
    enum V { I(i64), S(String), L(Vec<V>) }
    fn combine(a: &V, b: &V) -> V {
        match (a, b) {
            (V::I(x), V::I(y)) => V::I(x + y),
            (V::S(s), V::I(n)) | (V::I(n), V::S(s)) => V::S(format!("{}{}", s, n)),
            (V::S(x), V::S(y)) => V::S(x.clone() + y),
            (V::L(xs), other) | (other, V::L(xs)) => V::L(xs.iter().map(|x| combine(x, other)).collect()),
        }
    }
    fn bump_all(v: &mut V) {
        match v {
            V::I(n) => *n += 1,
            V::S(s) => s.push('+'),
            V::L(xs) => xs.iter_mut().for_each(bump_all),
        }
    }
    pub fn main() {
        let a = V::L(vec![V::I(1), V::S("s".into())]);
        let mut r = combine(&a, &V::I(10));
        bump_all(&mut r);
        let at @ V::L(_) = &r else { panic!() };
        println!("{:?} {}", r, at == &r);
    }
}

fn main() {
    p05_match_mut_enum_split::main();
    p47_enum_methods_mut_self::main();
    p56_match_on_tuple_refs_and_bindings::main();
}
