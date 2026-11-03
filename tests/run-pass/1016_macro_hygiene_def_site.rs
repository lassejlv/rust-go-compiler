// macro_rules! hygiene: local variables and labels named by a macro body
// resolve where the macro is defined (shadowing at the call site does not
// capture them), format-string captures follow the literal's context.
macro_rules! outer_def {
    ($name:ident) => {
        let base = 100;
        macro_rules! $name { () => { base + 1 } }
    };
}

macro_rules! fmt_cap {
    () => {{ let v = 3; format!("{v}") }};
}

macro_rules! fmt_lit {
    ($s:literal) => {{ let v = 1; let _ = v; format!($s) }};
}

macro_rules! twice {
    ($e:expr) => {{ let tmp = $e; tmp + tmp }};
}

fn generic<T: std::fmt::Debug + Clone>(t: T) -> String {
    let t2 = t.clone();
    macro_rules! show { () => { format!("{:?}", t2) } }
    let t2 = 5;
    format!("{} {}", show!(), t2)
}

fn param(p: i32) -> i32 {
    macro_rules! gp { () => { p * 2 } }
    let p = 1000;
    gp!() + p
}

struct S { f: i32 }
impl S {
    fn m(&self) -> i32 {
        let k = self.f;
        macro_rules! gk { () => { k + self.f } }
        let k = 0;
        gk!() + k
    }
}

fn main() {
    // basic
    let x = 1;
    macro_rules! gx { () => { x } }
    let x = 2;
    println!("{} {}", gx!(), x);
    // nested block + closure + loops
    {
        let x = 3;
        let c = |x: i32| gx!() * 10 + x;
        println!("{}", c(4));
        for x in 5..6 { println!("{} {}", gx!(), x); }
        let mut it = vec![7].into_iter();
        while let Some(x) = it.next() { println!("{} {}", gx!(), x); }
        let Some(x) = Some(8) else { return };
        println!("{} {}", gx!(), x);
        match Some(9) { Some(x) if gx!() < x => println!("arm {} {}", gx!(), x), _ => {} }
    }
    // macro defined by a macro, referring to the outer expansion's local
    outer_def!(gb);
    let base = 7;
    println!("{} {}", gb!(), base);
    // macro inside macro body referencing a local of the same expansion
    let r = {
        let y = 10;
        macro_rules! gy { () => { y * 2 } }
        let y = 20;
        macro_rules! gy2 { () => { gy!() + y } }
        let y = 30;
        gy2!() + y
    };
    println!("{}", r);
    // format captures
    let v = 2;
    println!("{} {} {}", fmt_cap!(), fmt_lit!("{v}"), v);
    // expression args keep caller hygiene
    let tmp = 4;
    println!("{}", twice!(tmp + 1));
    println!("{}", generic(vec![1]));
    println!("{}", param(3));
    println!("{}", S { f: 6 }.m());
    // redefinition of the same macro name
    let z = 1;
    macro_rules! gz { () => { z } }
    let z = 2;
    let a = gz!();
    macro_rules! gz { () => { z } }
    let z = 3;
    println!("{} {} {}", a, gz!(), z);
    // recursion
    let n = 5;
    macro_rules! rec { () => { n }; (+ $($t:tt)*) => { 1 + rec!($($t)*) } }
    let n = 50;
    println!("{} {}", rec!(+ + +), n);
    // local macro used inside another local macro defined later
    let q = 1;
    macro_rules! gq { () => { q } }
    let q = 2;
    macro_rules! gq2 { () => { gq!() + q } }
    let q = 3;
    println!("{} {}", gq2!(), q);
    // closure defined in macro body capturing def-site local
    let w = 11;
    macro_rules! mk { () => { move || w + 1 } }
    let w = 0;
    let f = mk!();
    println!("{} {}", f(), w);
    labels();
}

macro_rules! inner_loop {
    ($body:block) => {
        'a: for i in 0..3 {
            if i == 1 { continue 'a; }
            $body
        }
    };
}

macro_rules! brk_val {
    ($v:expr) => { 'b: { if $v > 2 { break 'b 1; } 0 } };
}

fn labels() {
    let mut log = vec![];
    'a: for j in 0..3 {
        inner_loop!({ log.push(j); if j == 1 { break 'a; } });
    }
    println!("{:?}", log);
    let mut n = 0;
    'outer: loop {
        macro_rules! out { () => { break 'outer } }
        loop {
            n += 1;
            if n > 3 { out!(); }
        }
    }
    println!("{}", n);
    println!("{} {}", brk_val!(5), brk_val!(1));
    let r = 'b: {
        let x = brk_val!(9);
        if x == 1 { break 'b 10; }
        20
    };
    println!("{}", r);
}
