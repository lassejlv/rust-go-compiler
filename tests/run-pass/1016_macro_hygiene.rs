macro_rules! swap_vals {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

macro_rules! make_var {
    ($name:ident, $val:expr) => { let $name = $val; };
}

macro_rules! hidden_var {
    () => { let secret = 99; let _ = secret; };
}

macro_rules! use_outer {
    ($e:expr) => {{ let tmp = 1000; $e + tmp }};
}

macro_rules! loop_break {
    ($body:block) => { 'outer: loop { $body; break 'outer; } };
}

macro_rules! counter_fn {
    ($name:ident) => {
        fn $name() -> u32 {
            static mut CALLS: u32 = 0;
            unsafe { CALLS += 1; CALLS }
        }
    };
}
counter_fn!(tick_a);
counter_fn!(tick_b);

macro_rules! closure_capture {
    ($x:ident) => { move || $x * 2 };
}

macro_rules! debug_expr {
    ($($e:expr),*) => { $( println!("{} = {:?}", stringify!($e), $e); )* };
}

macro_rules! early_return {
    ($opt:expr) => { match $opt { Some(v) => v, None => return -1 } };
}

fn find_sum(a: Option<i32>, b: Option<i32>) -> i32 {
    let x = early_return!(a);
    let y = early_return!(b);
    x + y
}

fn main() {
    let mut a = 1;
    let mut b = 2;
    let tmp = "outer tmp";
    swap_vals!(a, b);
    println!("a={} b={} tmp={}", a, b, tmp);
    make_var!(created, 42);
    println!("created={}", created);
    let secret = 1;
    hidden_var!();
    println!("secret={}", secret);
    let tmp = 5;
    println!("{}", use_outer!(tmp * 2));
    let mut n = 0;
    loop_break!({ n += 1; });
    println!("n={}", n);
    tick_a(); tick_a();
    println!("ticks {} {}", tick_a(), tick_b());
    let base = 21;
    let f = closure_capture!(base);
    println!("{}", f());
    let v = vec![1, 2];
    debug_expr!(1 + 2, v.len(), "s".repeat(3), (a, b));
    println!("{} {}", find_sum(Some(2), Some(3)), find_sum(None, Some(1)));
    let x = 10;
    macro_rules! local_uses_x { () => { x + 1 }; }
    println!("local {}", local_uses_x!());
    {
        let x = 50;
        println!("shadowed local {}", local_uses_x!() + x);
    }
}
