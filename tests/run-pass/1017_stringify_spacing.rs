// stringify! spacing like rustc: source spacing of tokens and delimited
// groups (`{x}` vs `{ x }`), macro body tokens printed `Alone`, fragments,
// and line wrapping of long output at 78 columns.
macro_rules! se { ($e:expr) => { stringify!($e) } }
macro_rules! si { ($i:item) => { stringify!($i) } }
macro_rules! st { ($($t:tt)*) => { stringify!($($t)*) } }
macro_rules! sty { ($t:ty) => { stringify!($t) } }
macro_rules! sb { ($b:block) => { stringify!($b) } }
macro_rules! sp { ($p:pat) => { stringify!($p) } }
macro_rules! ss { ($s:stmt) => { stringify!($s) } }
macro_rules! body { () => { stringify!({a} {b }{ c} { d }) } }
macro_rules! body2 { () => { stringify!(f({x}) a+b a + b {x+1}) } }
macro_rules! nest { ($e:expr) => { se!($e) } }
macro_rules! nestb { ($($t:tt)*) => { st!({$($t)*}) } }
macro_rules! se5 { ($e:expr) => { stringify!($e) } }
fn main() {
    println!("{}", stringify!({e}));
    println!("{}", stringify!({ e }));
    println!("{}", stringify!({e }));
    println!("{}", stringify!({ e}));
    println!("{}", stringify!({}));
    println!("{}", stringify!({ }));
    println!("{}", stringify!(()));
    println!("{}", stringify!(( )));
    println!("{}", stringify!([ ]));
    println!("{}", stringify!(( a )));
    println!("{}", stringify!([ a , b ]));
    println!("{}", stringify!({a + b}));
    println!("{}", stringify!(f({x})));
    println!("{}", stringify!(a{b}c));
    println!("{}", stringify!(a {b} c));
    println!("{}", stringify!(a+b));
    println!("{}", stringify!(a +b));
    println!("{}", stringify!(a+ b));
    println!("{}", stringify!(a<b>c));
    println!("{}", stringify!(x=>y));
    println!("{}", stringify!(x  =>  y));
    println!("{}", stringify!(a::b ::c:: d));
    println!("{}", stringify!(&&a & &b));
    println!("{}", stringify!(a,b , c ,d));
    println!("{}", stringify!(a;b ; c));
    println!("{}", stringify!(a.b . c .d));
    println!("{}", stringify!(f (x) f(x) if(x) let(a)));
    println!("{}", stringify!(# [a] #[b] #![c]));
    println!("{}", stringify!($a $ b));
    println!("{}", stringify!(x{}y));
    println!("{}", stringify!('a: loop {}));
    println!("{}", stringify!(-1 - 1 --1));
    println!("{}", stringify!(a=b==c!=d));
    println!("{}", stringify!(m!(x) m ! (x) m!{x} m![x]));
    println!("{}", stringify!(fn f(){} fn g() {}));
    println!("{}", stringify!(struct A{x:u8}));
    println!("{}", stringify!(1.0.max(2.0)));
    println!("{}", stringify!(x.0.1));
    println!("{}", se!(if a {b} else {c}));
    println!("{}", se!(if a { b } else { c }));
    println!("{}", se!(a+b));
    println!("{}", se!(f( x ,y )));
    println!("{}", se!({e}));
    println!("{}", se!(S{x:1}));
    println!("{}", se!(S { x: 1 }));
    println!("{}", se!(|x|x+1));
    println!("{}", se!(a[0]));
    println!("{}", si!(struct A{x:u8}));
    println!("{}", si!(fn f(){}));
    println!("{}", si!(fn f() { 1 }));
    println!("{}", sty!(Vec<u8>));
    println!("{}", sty!(Vec< u8 >));
    println!("{}", sty!(&'a mut[u8;3]));
    println!("{}", sb!({x}));
    println!("{}", sb!({ x }));
    println!("{}", sp!(Some(x)|None));
    println!("{}", ss!(let x=5));
    println!("{}", st!({e}));
    println!("{}", st!({ e } a+b {x}));
    println!("{}", body!());
    println!("{}", body2!());
    println!("{}", nest!({e}));
    println!("{}", nest!(a+b));
    println!("{}", nestb!(e));
    println!("{}", nestb!( e ));
    println!("{}", stringify!(a /* c */ b/*c*/c));
    println!("{}", stringify!(a // c
    b));
    println!("{}", stringify!(fn f() { let x = 1; let y = 2; let zzzzzzzzzzzzzzzzz = 3; let wwwwwwwwwwwwwwwwwwwwwwww = 4; x + y }));
    println!("{}", stringify!(fn f() {let x = 1; let y = 2; let zzzzzzzzzzzzzzzzz = 3; let wwwwwwwwwwwwwwwwwwwwwwww = 4; x + y}));
    println!("{}", stringify!(aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc (dddddddddddd, eeeeeeeeeeeeeee, fffffffffffffffffff, ggggggggggggg, hhhhhhhhhhhhhhh)));
    println!("{}", stringify!(aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc [dddddddddddd, eeeeeeeeeeeeeee, fffffffffffffffffff, ggggggggggggg, hhhhhhhhhhhhhhh] x y z));
    println!("{}", stringify!(aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc dddddddddddd eeeeeeeeeeeeeee fffffffffffffffffff ggggggggggggg hhhhhhhhhhhhhhh iiiiiiiiiiiiiii jjjjjjjjjjjjjjjjjj kkkkkkkkkkkkkkkkkk llllllllllll));
    println!("{}", se5!(aaaaaaaaaaaaaaa + bbbbbbbbbbbbbbbbbbbbbbbbbb + cccccccccccccccccccccccc + dddddddddddddddddddd + eeeeeeeeeeeee));
    println!("{}", stringify!(x { aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc { dddddddddddd eeeeeeeeeeeeeee fffffffffffffffffff } ggggggggggggg hhhhhhhhhhhhhhh iiiiiiiiiiiiiii } y));
    println!("{}", stringify!(xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx yyy));
    println!("{}", stringify!(a {} b {} c {} aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc dddddddddddd eeeeeeeeeeeeeee {} fffffffffffffffffff));
    println!("{}", stringify!(a {x} b {y} c {z} aaaaaaaaaa bbbbbbbbbbbbbbb ccccccccccccccc dddddddddddd eeeeeeeeeeeeeee {w} fffffffffffffffffff));
}
