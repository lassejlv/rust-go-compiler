// textual scoping of macro_rules!: blocks, nested fns, shadowing std macros, #[macro_use] chains
fn early() -> i32 {
    macro_rules! helper { () => { 1 }; }
    let a = helper!();
    {
        macro_rules! helper { () => { 10 }; }
        let b = helper!();
        let _ = b;
    }
    a + helper!()
}

macro_rules! helper { () => { 2 }; }

fn late() -> i32 { helper!() }

fn with_nested_fn() -> i32 {
    macro_rules! sq { ($x:expr) => { $x * $x }; }
    fn inner(v: i32) -> i32 { sq!(v) + 1 }
    let c = |v: i32| { macro_rules! sq { ($x:expr) => { $x + $x }; } sq!(v) };
    inner(3) + c(4) + sq!(2)
}

mod shadow_std {
    macro_rules! vec { ($($x:expr),*) => { [$($x),*].len() }; }
    pub fn count() -> usize { vec![1, 2, 3] }
    macro_rules! assert_eq { ($a:expr, $b:expr) => { $a + $b }; }
    pub fn sum() -> i32 { assert_eq!(2, 3) }
}

#[macro_use]
mod outer {
    #[macro_use]
    mod inner {
        macro_rules! from_inner { () => { "inner" }; }
    }
    macro_rules! from_outer { () => { concat!("outer+", from_inner!()) }; }
    pub fn use_inner() -> &'static str { from_inner!() }
}

fn loops() -> Vec<i32> {
    let mut v = Vec::new();
    for i in 0..3 {
        macro_rules! push_scaled { ($k:expr) => { v.push(i * $k) }; }
        push_scaled!(10);
        if i == 1 {
            macro_rules! push_scaled { ($k:expr) => { v.push(-$k) }; }
            push_scaled!(5);
        }
        push_scaled!(100);
    }
    v
}

fn main() {
    println!("{} {}", early(), late());
    println!("{}", with_nested_fn());
    println!("{} {}", shadow_std::count(), shadow_std::sum());
    println!("{} {} {}", outer::use_inner(), from_inner!(), from_outer!());
    println!("{:?}", loops());
    println!("{}", helper!());
    let v = vec![1, 2];
    println!("{:?}", v);
}
