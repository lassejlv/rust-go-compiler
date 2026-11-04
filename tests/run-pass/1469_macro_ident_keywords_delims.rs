// ident fragments matching keywords (self, raw idents), delimiters of invocations, path separators
macro_rules! field_of { ($obj:ident . $f:ident) => { $obj.$f }; }
macro_rules! call_on_self { ($s:ident, $m:ident) => { $s.$m() }; }
macro_rules! kw_name { ($i:ident) => { stringify!($i) }; }
macro_rules! join_path { ($($seg:ident)::+) => { concat!($(stringify!($seg), "/"),+) }; }
macro_rules! use_path { ($($seg:ident)::+) => { $($seg)::+ }; }
macro_rules! alt_pats { ($e:expr, $($p:pat_param)|*) => { match $e { $($p)|* => "hit", #[allow(unreachable_patterns)] _ => "miss" } }; }
macro_rules! tail_expr { () => { 40 + 2 }; }
macro_rules! stmts_then_expr { ($x:ident) => { let $x = 3; let _y = $x; }; }

struct S { v: i32 }
impl S {
    fn get(&self) -> i32 { field_of!(self.v) }
    fn twice(&self) -> i32 { call_on_self!(self, get) * 2 }
}

mod deep { pub mod er { pub fn f() -> &'static str { "deep::er::f" } pub const C: i32 = 9; } }

fn braced() -> i32 { tail_expr!{} }
fn bracketed() -> i32 { tail_expr![] }

fn main() {
    let s = S { v: 21 };
    println!("{} {}", s.get(), s.twice());
    println!("{} {} {} {}", kw_name!(self), kw_name!(fn), kw_name!(r#try), kw_name!(Self));
    println!("{}", join_path!(std::collections::hash_map));
    println!("{} {}", use_path!(deep::er::f)(), use_path!(deep::er::C));
    println!("{} {}", alt_pats!(3, 1 | 2 | 3), alt_pats!('z', 'a'..='f' | 'x'));
    println!("{} {}", braced(), bracketed());
    stmts_then_expr!(z);
    println!("{}", z);
    let v = vec!{1, 2};
    let w = vec!(3, 4);
    println!("{:?} {:?}", v, w);
    println!{"{}", "braced println"};
    println!["{}", "bracketed println"];
    let r = { tail_expr!() };
    println!("{}", r);
    let q = if true { tail_expr!{} } else { 0 };
    println!("{}", q);
}
