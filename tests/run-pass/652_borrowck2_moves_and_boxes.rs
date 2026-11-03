// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p18_move_and_reinit {
    fn consume(s: String) -> usize { s.len() }
    pub fn main() {
        let mut s = String::from("abc");
        let mut total = 0;
        for i in 0..3 {
            total += consume(s);
            s = format!("{}{}", "x".repeat(i), i);
        }
        let v = vec![s];
        let s = 5;
        let mut o = Some(String::from("q"));
        if let Some(inner) = o.take() { total += inner.len(); }
        o = Some("rr".into());
        let r;
        let cond = total > 3;
        let moved = String::from("m");
        if cond { r = moved; } else { r = String::from("n"); drop(moved); }
        println!("{} {:?} {} {:?} {}", total, v, s, o, r);
    }
}

mod p35_box_moves {
    #[derive(Debug)]
    struct Big { name: String, data: Vec<u8> }
    pub fn main() {
        let b = Box::new(Big { name: "n".into(), data: vec![1, 2] });
        let Big { name, data } = *b;
        let bt = Box::new((String::from("x"), String::from("y")));
        let (x, y) = *bt;
        let bb = Box::new(Big { name: "m".into(), data: vec![] });
        let nm = bb.name;
        let rest = bb.data;
        let mut bx: Box<Vec<i32>> = Box::new(vec![]);
        bx.push(1);
        let r: &mut Vec<i32> = &mut bx;
        r.push(2);
        let v: Vec<i32> = *bx;
        let s: Box<str> = "boxed".into();
        let len = s.len();
        let st: String = s.into_string();
        println!("{} {:?} {} {} {} {:?} {:?} {} {}", name, data, x, y, nm, rest, v, len, st);
    }
}

mod p14_mem_take_replace_self {
    #[derive(Debug, Default)]
    struct Buf { pending: Vec<String>, done: Vec<String>, cur: String }
    impl Buf {
        fn flush(&mut self) {
            let items = std::mem::take(&mut self.pending);
            for it in items { self.done.push(it); }
            let old = std::mem::replace(&mut self.cur, String::from("new"));
            self.done.push(old);
        }
        fn drain_into(&mut self) {
            while let Some(x) = self.pending.pop() {
                if x.len() < 3 { self.pending.push(format!("{}x", x)); } else { self.done.push(x); }
            }
        }
    }
    pub fn main() {
        let mut b = Buf { pending: vec!["a".into(), "b".into()], ..Default::default() };
        b.cur = "c".into();
        b.flush();
        b.pending.push("q".into());
        b.drain_into();
        println!("{:?}", b);
    }
}

fn main() {
    p18_move_and_reinit::main();
    p35_box_moves::main();
    p14_mem_take_replace_self::main();
}
