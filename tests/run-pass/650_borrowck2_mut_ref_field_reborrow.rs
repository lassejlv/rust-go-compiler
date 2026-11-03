// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p33_mut_ref_field_struct {
    struct Cursor<'a, T> { data: &'a mut Vec<T>, pos: usize }
    impl<'a, T: Clone> Cursor<'a, T> {
        fn current(&mut self) -> &mut T { &mut self.data[self.pos] }
        fn advance(&mut self) -> bool { self.pos += 1; self.pos < self.data.len() }
        fn data(&mut self) -> &mut Vec<T> { self.data }
        fn into_current(self) -> &'a mut T { &mut self.data[self.pos] }
    }
    struct Guard<'a> { log: &'a mut Vec<String>, name: &'static str }
    impl<'a> Drop for Guard<'a> { fn drop(&mut self) { self.log.push(format!("drop {}", self.name)); } }
    pub fn main() {
        let mut v = vec![1, 2, 3];
        {
            let mut c = Cursor { data: &mut v, pos: 0 };
            loop { *c.current() *= 10; if !c.advance() { break; } }
            c.pos = 0;
            c.data().push(4);
            let last = c.into_current();
            *last += 1;
        }
        let mut log = Vec::new();
        {
            let g = Guard { log: &mut log, name: "a" };
            g.log.push("in".into());
        }
        {
            let _g = Guard { log: &mut log, name: "b" };
        }
        log.push("end".into());
        println!("{:?} {:?}", v, log);
    }
}

fn main() {
    p33_mut_ref_field_struct::main();
}
