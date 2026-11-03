// stringify! of substituted macro fragments: `tt` keeps its spacing,
// `ident` prints as a separate token, other fragments as invisible groups;
// two identifiers are never glued.
macro_rules! twot { ($a:tt, $b:tt) => { stringify!($a $b) } }
macro_rules! twot2 { ($a:tt $b:tt) => { stringify!($a $b) } }
macro_rules! seq { ($($t:tt)*) => { stringify!($($t)*) } }
macro_rules! seq2 { ($($t:tt)*) => { stringify!($($t)* ; $($t)*) } }
macro_rules! rev { ($a:tt $b:tt) => { stringify!($b $a) } }
macro_rules! id2 { ($a:ident $b:tt) => { stringify!($a $b) } }
macro_rules! twot4 { ($a:tt, $b:tt) => { stringify!($a $b) } }
macro_rules! twoi { ($a:tt, $b:ident) => { stringify!($a $b) } }
macro_rules! twol { ($a:tt, $b:lifetime) => { stringify!($a $b) } }
macro_rules! twoe { ($a:tt, $b:expr) => { stringify!($a $b) } }
macro_rules! twox { ($a:tt ,) => { stringify!($a x) } }
fn main() {
    println!("{}", twot!(a, 1));
    println!("{}", twot!(1, a));
    println!("{}", twot!(1, 2));
    println!("{}", twot!('x', a));
    println!("{}", twot!(a, 'b));
    println!("{}", twot!(a, "s"));
    println!("{}", twot!(+, =));
    println!("{}", twot!(-, >));
    println!("{}", twot!(=, =));
    println!("{}", twot!(+, a));
    println!("{}", twot!(a, +));
    println!("{}", twot!(a, (b)));
    println!("{}", twot!((a), b));
    println!("{}", twot!([a], b));
    println!("{}", twot!(a, [b]));
    println!("{}", twot!(a, {b}));
    println!("{}", twot2!(a+));
    println!("{}", twot2!(a.));
    println!("{}", twot2!(a(x)));
    println!("{}", twot2!((x)a));
    println!("{}", twot2!('a,));
    println!("{}", seq!(a+b));
    println!("{}", seq!(a + b));
    println!("{}", seq!((a)b));
    println!("{}", seq2!(x));
    println!("{}", seq2!(x y));
    println!("{}", seq2!(+));
    println!("{}", seq2!(a+));
    println!("{}", rev!(a+));
    println!("{}", rev!(+a));
    println!("{}", rev!(a.));
    println!("{}", rev!(.a));
    println!("{}", rev!(=+));
    println!("{}", rev!(1+));
    println!("{}", id2!(a+));
    println!("{}", id2!(a.));
    println!("{}", id2!(a(x)));
    println!("{}", twot4!('a, b));
    println!("{}", twot4!(a, r#b));
    println!("{}", twot4!(r#a, b));
    println!("{}", twot4!(a, as));
    println!("{}", twot4!(as, a));
    println!("{}", twot4!(true, a));
    println!("{}", twot4!(a, true));
    println!("{}", twot4!(_, a));
    println!("{}", twot4!(a, _));
    println!("{}", twot4!(fn, f));
    println!("{}", twot4!(a, 'b'));
    println!("{}", twot4!("a", "b"));
    println!("{}", twot4!(1, 2u8));
    println!("{}", twoi!(a, b));
    println!("{}", twoi!(+, b));
    println!("{}", twol!(a, 'b));
    println!("{}", twoe!(a, b));
    println!("{}", twoe!(a, 1));
    println!("{}", twoe!(+, 1));
    println!("{}", twoe!(a, b+c));
    println!("{}", twox!(a,));
    println!("{}", twox!(+,));
    println!("{}", twox!(1,));
    println!("{}", twox!('a,));
    println!("{}", twox!((a),));
}
