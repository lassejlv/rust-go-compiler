// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p19_closures_fnmut_boxed {
    struct Machine { handlers: Vec<Box<dyn FnMut(i32) -> i32>>, acc: i32 }
    impl Machine {
        fn run(&mut self, x: i32) {
            for h in self.handlers.iter_mut() { self.acc += h(x); }
        }
    }
    fn make_counter() -> impl FnMut() -> u32 { let mut c = 0; move || { c += 1; c } }
    pub fn main() {
        let mut calls = 0;
        {
            let mut f = |x: i32| { calls += 1; x * 2 };
            let a = f(1); let b = f(2);
            println!("{} {}", a, b);
        }
        let mut m = Machine { handlers: vec![], acc: 0 };
        let mut seen = 0;
        m.handlers.push(Box::new(move |x| { seen += x; seen }));
        m.handlers.push(Box::new(|x| x * 100));
        m.run(1); m.run(2);
        let mut c = make_counter();
        c(); c();
        println!("{} {} {}", calls, m.acc, c());
    }
}

mod p20_thread_scope_borrows {
    use std::thread;
    pub fn main() {
        let mut a = vec![1, 2, 3];
        let mut b = vec![10, 20];
        let shared = String::from("s");
        thread::scope(|sc| {
            sc.spawn(|| { a.push(4); a.push(shared.len() as i32); });
            sc.spawn(|| { b.iter_mut().for_each(|x| *x += 1); println!("{}", shared); });
        });
        let mut parts = vec![0u64; 4];
        thread::scope(|sc| {
            for (i, p) in parts.iter_mut().enumerate() {
                sc.spawn(move || *p = (i as u64 + 1) * 10);
            }
        });
        println!("{:?} {:?} {:?}", a, b, parts);
    }
}

fn main() {
    p19_closures_fnmut_boxed::main();
    p20_thread_scope_borrows::main();
}
