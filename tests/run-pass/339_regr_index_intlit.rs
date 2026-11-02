mod private { pub trait Sealed {} impl Sealed for usize {} impl Sealed for str {} impl<'a, T: ?Sized + Sealed> Sealed for &'a T {} }
pub trait Idx: private::Sealed { fn get<'v>(&self, v: &'v Val) -> &'v Val; }
#[derive(Debug)] pub enum Val { Arr(Vec<Val>), Num(i64), Null }
static NULL: Val = Val::Null;
impl Idx for usize { fn get<'v>(&self, v: &'v Val) -> &'v Val { match v { Val::Arr(a) => a.get(*self).unwrap_or(&NULL), _ => &NULL } } }
impl Idx for str { fn get<'v>(&self, _v: &'v Val) -> &'v Val { &NULL } }
impl<'a, T: ?Sized + Idx> Idx for &'a T { fn get<'v>(&self, v: &'v Val) -> &'v Val { (**self).get(v) } }
impl<I: Idx> std::ops::Index<I> for Val { type Output = Val; fn index(&self, i: I) -> &Val { i.get(self) } }
fn main() {
    let v = Val::Arr(vec![Val::Num(1), Val::Arr(vec![Val::Num(7), Val::Num(8)])]);
    println!("{:?} {:?} {:?}", v[1][1], v["x"], v[0]);
}
