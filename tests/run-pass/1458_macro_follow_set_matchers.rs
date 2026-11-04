// follow-set compliant matchers with tricky separators and ambiguity-free choices
macro_rules! ty_list {
    ($($t:ty),* ; $($u:ty)=>*) => { ($(std::mem::size_of::<$t>()),*, $(std::mem::size_of::<$u>()),*) };
}
macro_rules! path_then_block {
    ($p:path { $($body:tt)* }) => { { let v: $p = <$p>::from($($body)*); v } };
}
macro_rules! pat_if {
    ($e:expr => $p:pat if $g:expr) => { match $e { $p if $g => "guarded", _ => "nope" } };
}
macro_rules! expr_semis {
    ($($e:expr);+) => { [$($e),+] };
}
macro_rules! ident_opts {
    ($i:ident $(as $alias:ident)? $(: $t:ty)?) => {
        concat!(stringify!($i) $(, " as ", stringify!($alias))? $(, ": ", stringify!($t))?)
    };
}
macro_rules! lifetimes {
    ($($l:lifetime),*) => { [$(stringify!($l)),*] };
}
macro_rules! vis_items {
    ($($v:vis $name:ident),*) => { [$(concat!(stringify!($v), "|", stringify!($name))),*] };
}
macro_rules! block_then {
    ($b:block $e:expr) => { $b + $e };
}
macro_rules! stmt_list {
    ($($s:stmt)*) => { { $($s;)* } };
}
macro_rules! tt_bracketed {
    ($( [$($inner:tt)*] )*) => { [$(stringify!($($inner)*)),*] };
}
macro_rules! alternatives {
    (add $a:expr, $b:expr) => { $a + $b };
    (mul $a:expr, $b:expr) => { $a * $b };
    (neg $a:expr) => { -$a };
    ($other:ident $($rest:tt)*) => { concat!("unknown op ", stringify!($other)) };
}
macro_rules! some_or_none {
    () => { None::<i32> };
    ($v:literal) => { Some($v) };
}

fn main() {
    println!("{:?}", ty_list!(u8, (u16, u32), [u64; 2]; i8 => Option<Box<u8>>));
    println!("{}", path_then_block!(String { "abc" }));
    println!("{} {}", pat_if!(5 => x if x > 3), pat_if!(2 => x if x > 3));
    println!("{:?}", expr_semis!(1 + 1; 2 * 3; { 4 }));
    println!("{}", ident_opts!(foo));
    println!("{}", ident_opts!(foo as bar));
    println!("{}", ident_opts!(foo: Vec<u8>));
    println!("{}", ident_opts!(foo as bar: (i32, i32)));
    println!("{:?}", lifetimes!('a, 'static, '_));
    println!("{:?}", vis_items!(pub a, b, pub(crate) c, pub(super) d));
    println!("{}", block_then!({ 1 } 2));
    stmt_list!(let a = 1 let b = a + 1 println!("{}", a + b));
    println!("{:?}", tt_bracketed!([a b] [] [1, 2] [(x)]));
    println!("{} {} {} {}", alternatives!(add 1, 2), alternatives!(mul 3, 4), alternatives!(neg 5), alternatives!(div 1, 2));
    println!("{:?} {:?}", some_or_none!(), some_or_none!(4));
}
