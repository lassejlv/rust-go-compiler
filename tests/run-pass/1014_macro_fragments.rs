macro_rules! classify {
    ($l:literal) => { format!("literal {}", stringify!($l)) };
    ($b:block) => { format!("block -> {}", $b) };
    ($i:ident) => { format!("ident {}", stringify!($i)) };
    ($e:expr) => { format!("expr {}", stringify!($e)) };
}

macro_rules! matches_pat {
    ($v:expr, $($p:pat_param)|+) => { match $v { $($p)|+ => true, _ => false } };
}

macro_rules! pat2021 {
    ($v:expr, $p:pat) => { if let $p = $v { "yes" } else { "no" } };
}

macro_rules! sized {
    ($($t:ty),*) => { vec![$((stringify!($t), std::mem::size_of::<$t>())),*] };
}

macro_rules! with_lifetime {
    ($lt:lifetime, $name:ident) => {
        struct $name<$lt> { s: &$lt str }
        impl<$lt> $name<$lt> {
            fn get(&self) -> &$lt str { self.s }
        }
    };
}
with_lifetime!('a, Holder);

mod m {
    macro_rules! vis_struct {
        ($v:vis $name:ident) => { $v struct $name(pub i32); };
    }
    vis_struct!(pub Public);
    vis_struct!(Private);
    pub fn make_private() -> i32 { Private(5).0 }
}

macro_rules! item_list {
    ($($it:item)*) => { $($it)* };
}
item_list! {
    fn gen_a() -> i32 { 1 }
    const GEN_B: i32 = 2;
    struct GenC;
}

macro_rules! stmts {
    ($($s:stmt;)*) => {{ $($s;)* }};
}

macro_rules! path_call {
    ($p:path) => { $p(3) };
}

macro_rules! meta_attr {
    (#[$m:meta] $name:ident) => { #[$m] struct $name { v: u8 } };
}
meta_attr!(#[derive(Debug, Default)] Metad);

macro_rules! tt_group {
    ($($t:tt)*) => { vec![$(stringify!($t)),*] };
}

fn triple(x: i32) -> i32 { x * 3 }

fn main() {
    println!("{}", classify!(42));
    println!("{}", classify!("str"));
    println!("{}", classify!({ 1 + 2 }));
    println!("{}", classify!(foo));
    println!("{}", classify!(a + b * 2));
    println!("{}", classify!(-5));
    let v = Some(7);
    println!("{} {}", matches_pat!(v, Some(1..=9)), matches_pat!(3, 0 | 1 | 2));
    println!("{}", matches_pat!((1, "x"), (1, _) | (_, "y")));
    println!("{} {}", pat2021!(Some(3), Some(3) | None), pat2021!(Some(4), Some(3) | None));
    println!("{:?}", sized!(u8, (u16, u8), [u32; 3], &str, Option<Box<i32>>));
    let h = Holder { s: "life" };
    println!("{}", h.get());
    println!("{} {}", m::Public(9).0, m::make_private());
    println!("{} {}", gen_a(), GEN_B);
    let _ = GenC;
    let r = stmts! {
        let a = 5;
        let b = a * 2;
        println!("in stmts {}", a + b);
    };
    let _: () = r;
    println!("{}", path_call!(triple));
    println!("{}", path_call!(i32::abs));
    println!("{:?}", Metad::default());
    println!("{:?}", tt_group!(a (b c) [d] {e} + ::f 'g' "h" 1.5));
    let _ = Metad { v: 1 }.v;
}
