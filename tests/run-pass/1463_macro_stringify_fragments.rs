// stringify! spacing of fragments and raw tokens passed through macros
macro_rules! s_expr { ($e:expr) => { stringify!($e) }; }
macro_rules! s_ty { ($t:ty) => { stringify!($t) }; }
macro_rules! s_pat { ($p:pat) => { stringify!($p) }; }
macro_rules! s_item { ($i:item) => { stringify!($i) }; }
macro_rules! s_tts { ($($t:tt)*) => { stringify!($($t)*) }; }
macro_rules! s_path { ($p:path) => { stringify!($p) }; }
macro_rules! s_mixed { ($a:ident $b:ty, $c:expr) => { stringify!($a: $b = $c;) }; }
macro_rules! s_lit { ($l:literal) => { stringify!($l) }; }
macro_rules! s_block { ($b:block) => { stringify!($b) }; }
macro_rules! s_vis { ($v:vis) => { stringify!($v struct X;) }; }
macro_rules! s_meta { ($m:meta) => { stringify!(#[$m]) }; }
macro_rules! s_lt { ($l:lifetime) => { stringify!(&$l str) }; }
macro_rules! s_stmt { ($s:stmt) => { stringify!($s) }; }

fn main() {
    println!("{}", s_expr!(a+b*c));
    println!("{}", s_expr!(f(x,y)[0].z));
    println!("{}", s_expr!(|a,b|a+b));
    println!("{}", s_expr!(Foo{a:1,..Default::default()}));
    println!("{}", s_expr!(&mut *x));
    println!("{}", s_expr!(a as u8 as i32));
    println!("{}", s_expr!(if a {b} else {c}));
    println!("{}", s_expr!(match x { Some(v) => v, None => 0 }));
    println!("{}", s_expr!(-x..=y));
    println!("{}", s_ty!(&'a mut Vec<Option<(u8,i8)>>));
    println!("{}", s_ty!(dyn Fn(i32)->i32+Send));
    println!("{}", s_ty!(impl Iterator<Item=u8>));
    println!("{}", s_ty!([u8;4]));
    println!("{}", s_ty!(<T as Trait>::Assoc));
    println!("{}", s_pat!(Some(x@1..=5)|None));
    println!("{}", s_pat!(Point{x,y:0,..}));
    println!("{}", s_pat!(&[first,..,last]));
    println!("{}", s_item!(pub fn f<T:Clone>(t:&T)->T where T:Copy{t.clone()}));
    println!("{}", s_item!(struct S<'a>{x:&'a str}));
    println!("{}", s_tts!(a+b*c));
    println!("{}", s_tts!(x::<u8>() . await ?));
    println!("{}", s_tts!(#[derive(Debug)] enum E{A,B(i32)}));
    println!("{}", s_tts!(a=>b, c<=d, e>>=f, g::h, i..j, k..=l, m->n));
    println!("{}", s_tts!('a: loop { break 'a; }));
    println!("{}", s_path!(std::collections::HashMap<K,V>));
    println!("{}", s_path!(::core::mem::swap));
    println!("{}", s_mixed!(x Vec<u8>, vec![1,2]));
    println!("{}", s_lit!(-12));
    println!("{}", s_lit!(r#"raw"#));
    println!("{}", s_block!({let a=1;a}));
    println!("{}", s_vis!(pub(crate)));
    println!("{}", s_meta!(cfg(all(unix,not(test)))));
    println!("{}", s_meta!(path="a.rs"));
    println!("{}", s_lt!('static));
    println!("{}", s_stmt!(let x:i32=5));
}
