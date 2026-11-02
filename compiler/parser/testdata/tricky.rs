use std::error::Error;
impl<T> From<T> for Foo where T: Into<String> { fn from(t: T) -> Self { Foo(t.into()) } }
pub(crate) struct PC;
pub(super) enum PS { A }
struct D<T = i32>(T);
enum Flags { A = 1 << 0, B = 1 << 1, C = -1 }
impl<T: ?Sized> Tr for Box<T> {}
impl Tr for fn(i32) -> i32 {}
type F = dyn for<'a> Fn(&'a str);
type G = Box<dyn Fn(&str) -> Result<(), Box<dyn Error>> + Send + 'static>;
unsafe extern "C" fn cb(x: i32) -> i32 { x }
extern "system" { fn sys(); }
trait Gat { type Item<'a> where Self: 'a; fn get<'a>(&'a self) -> Self::Item<'a>; }
impl X { pub const fn new() -> Self { X } fn g<T>(&self) where T: Default {} }
fn bounds<T: Clone +, U>() where U: Copy + , for<'a> &'a U: Into<u8> {}
fn never() -> ! { loop {} }
fn f() {
    #![allow(unused)]
    let x = &mut **y;
    if let [a, b] = v[..] {}
    let f = foo::<Vec<u8>>;
    let r = Ok::<(), String>(());
    let s = F { f: |x| x, g: move || 1 };
    let v: Vec<i32> = it.collect::<Result<Vec<_>, _>>()?;
    let a = b as u8 as char;
    if a as usize > b {}
    if x as i32 >= 0 {}
    let sh = x as u64 << 2;
    let c = || unreachable!();
    match x { Some(y) => y, None => panic!("no") }
    match x { #[cfg(x)] A => {} B => |x| x, }
    let arr: [u8; size_of::<u32>()] = [0; 4];
    foo::<{ 3 + 1 }>();
    (|| {})();
    return match x { _ => 1 };
    x = if c { 1 } else { 2 };
    matches!(self, Self::A | Self::B);
    match x {} (a, b) = (1, 2);
    *self.x.borrow_mut() += 1;
    let n = -1i32.pow(2);
    std::println!("x");
    let v = vec![0; n];
    'a: while let Some(x) = it.next() {}
    x.0 += 1;
    for i in 1..n+1 {}
    let l = t.0.len() + 1.5.max(2.0) as usize + x.0.0;
    let c = '\\';
    let s = "{{}}";
    while let Some(top) = stack.last() && *top > 0 {}
    let cl = async move |x| x;
    let z = &(a, b);
    let w = [[0u8; 3]; 3];
    let q = Foo::<T> { x };
    let tuple_struct = Wrapper(1).0;
    let b = (a.len() as i32) < 3;
    let idx = arr[i as usize] as i64;
    if x.is_some() && y.is_none() { }
    let mut it = v.iter().peekable();
    while let Some(&c) = it.peek() { it.next(); }
    let m = x.map_or(0, |v| v + 1);
    let o = match a { A { b: B { c, .. }, .. } => c, };
    let s2: &[&str] = &["a", "b"];
    let p = &*x;
    let ptr = &x as *const i32 as usize;
    let g = |f: &dyn Fn(i32) -> i32| f(1);
    let h: fn() = || {};
    let t: (i32, (u8, char)) = (1, (2, 'c'));
    let (a, (b, c)) = t;
    let nested_closure = |x| move |y| x + y;
    let cmp = a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
    let neg_range = -5..5;
    match n { ..=0 => {}, 1..=9 => {}, 10.. => {} }
    let ret = loop { if x { break Some(1); } else { break None } };
    unsafe { COUNTER += 1 };
    { let inner = 1; }
    let label_val = 'l: loop { break 'l 42; };
    if let Some(ref mut x) = opt { *x += 1; }
    let tr = <T as Trait<U>>::CONST;
    let x = y.iter().map(|x| -> Result<i32, String> { Ok(*x) });
    let r = x?;
    let shadow = shadow.clone();
}
