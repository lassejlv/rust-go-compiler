use std::fmt::{self, Debug, Display};
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

trait Speak { fn speak(&self) -> String; }
trait SpeakMut { fn speak(&mut self) -> String; }

struct Bot { n: u32 }
impl Bot {
    fn speak(&self) -> String { format!("inherent {}", self.n) }
}
impl Speak for Bot { fn speak(&self) -> String { format!("trait {}", self.n) } }
impl SpeakMut for Bot { fn speak(&mut self) -> String { self.n += 1; format!("mut {}", self.n) } }

trait ByVal { fn which(self) -> &'static str; }
trait ByRef { fn which(&self) -> &'static str; }
impl ByVal for &u8 { fn which(self) -> &'static str { "ByVal for &u8" } }
impl ByRef for u8 { fn which(&self) -> &'static str { "ByRef for u8" } }

trait Len { fn len(&self) -> usize; }
impl<T> Len for Vec<T> { fn len(&self) -> usize { 999 } }

struct Smart<T>(T);
impl<T> Deref for Smart<T> { type Target = T; fn deref(&self) -> &T { &self.0 } }
impl<T> Smart<T> { fn get(&self) -> &'static str { "Smart::get" } }
struct Plain;
impl Plain { fn get(&self) -> &'static str { "Plain::get" } fn only_plain(&self) -> u8 { 7 } }

trait Greet { fn hi(&self) -> String { "default hi".into() } }
impl Greet for i32 {}
impl Greet for str { fn hi(&self) -> String { format!("str hi {}", self) } }
impl<T: Greet + ?Sized> Greet for Box<T> {}
impl<T: Greet> Greet for [T] { fn hi(&self) -> String { format!("slice of {}", self.len()) } }

trait Into2<T> { fn into2(self) -> T; }
impl Into2<String> for i32 { fn into2(self) -> String { format!("s{}", self) } }
impl Into2<f64> for i32 { fn into2(self) -> f64 { self as f64 + 0.5 } }

struct Meters(f64);
impl Display for Meters { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}m", self.0) } }
impl Debug for Meters { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Meters({:?})", self.0) } }

trait Tag { const TAG: &'static str; fn tag(&self) -> &'static str { Self::TAG } }
impl Tag for Meters { const TAG: &'static str = "meters"; }
impl<T: Tag> Tag for Vec<T> { const TAG: &'static str = "vec"; }
impl<T: Tag> Tag for Option<T> { const TAG: &'static str = "opt"; fn tag(&self) -> &'static str { match self { Some(x) => x.tag(), None => Self::TAG } } }

fn main() {
    let mut b = Bot { n: 1 };
    println!("{}", b.speak());
    println!("{}", Speak::speak(&b));
    println!("{}", SpeakMut::speak(&mut b));
    println!("{}", <Bot as SpeakMut>::speak(&mut b));
    println!("{}", Bot::speak(&b));

    let x: u8 = 5;
    println!("{}", ByVal::which(&x));
    println!("{}", ByRef::which(&x));

    let v = vec![1, 2, 3];
    println!("{} {}", v.len(), Len::len(&v));
    let rv = &v;
    println!("{}", rv.len());

    let s = Smart(Plain);
    println!("{} {} {}", s.get(), (*s).get(), s.only_plain());
    let ss = Smart(Smart(Plain));
    println!("{} {} {}", ss.get(), (**ss).get(), ss.only_plain());

    println!("{}", 5.hi());
    println!("{}", "abc".hi());
    let bs: Box<str> = "boxed".into();
    println!("{}", bs.hi());
    println!("{}", Greet::hi(&*bs));
    let arr = [1, 2, 3];
    println!("{}", arr[..].hi());
    println!("{}", Box::new(7).hi());

    let st: String = 3.into2();
    let fl: f64 = 3.into2();
    println!("{} {}", st, fl);
    println!("{}", <i32 as Into2<String>>::into2(9));

    let m = Meters(2.5);
    println!("{} {:?} {}", m, m, m.tag());
    println!("{} {} {}", vec![Meters(1.0)].tag(), Some(Meters(1.0)).tag(), None::<Meters>.tag());
    println!("{}", <Option<Vec<Meters>> as Tag>::TAG);

    let shared = Rc::new(RefCell::new(vec![String::from("a")]));
    shared.borrow_mut().push("b".into());
    let n = shared.borrow().iter().map(|s| s.len()).sum::<usize>();
    let cl = Rc::clone(&shared);
    cl.borrow_mut()[0].push('!');
    println!("{} {:?} {}", n, shared.borrow(), Rc::strong_count(&cl));
    let first_len = shared.borrow().first().map(String::len);
    println!("{:?}", first_len);
}
