// Custom smart pointers and guards: Deref/DerefMut with auto-deref method
// calls, deref coercions (&MyBox<String> -> &str), a RefCell-like guard with
// Drop releasing a borrow flag, copy-on-write pointer, and Index through Deref.
use std::cell::Cell;
use std::fmt;
use std::ops::{Deref, DerefMut};

struct MyBox<T>(Box<T>, Cell<u32>);
impl<T> MyBox<T> { fn new(v: T) -> Self { MyBox(Box::new(v), Cell::new(0)) } fn derefs(&self) -> u32 { self.1.get() } }
impl<T> Deref for MyBox<T> { type Target = T; fn deref(&self) -> &T { self.1.set(self.1.get() + 1); &self.0 } }
impl<T> DerefMut for MyBox<T> { fn deref_mut(&mut self) -> &mut T { self.1.set(self.1.get() + 100); &mut self.0 } }

struct Lock<T> { val: std::cell::UnsafeCell<T>, state: Cell<i32> } // >0 readers, -1 writer
struct ReadGuard<'a, T> { l: &'a Lock<T> }
struct WriteGuard<'a, T> { l: &'a Lock<T> }
impl<T> Lock<T> {
    fn new(v: T) -> Self { Lock { val: std::cell::UnsafeCell::new(v), state: Cell::new(0) } }
    fn read(&self) -> Option<ReadGuard<'_, T>> { if self.state.get() < 0 { None } else { self.state.set(self.state.get() + 1); Some(ReadGuard { l: self }) } }
    fn write(&self) -> Option<WriteGuard<'_, T>> { if self.state.get() != 0 { None } else { self.state.set(-1); Some(WriteGuard { l: self }) } }
}
impl<T> Deref for ReadGuard<'_, T> { type Target = T; fn deref(&self) -> &T { unsafe { &*self.l.val.get() } } }
impl<T> Drop for ReadGuard<'_, T> { fn drop(&mut self) { self.l.state.set(self.l.state.get() - 1); } }
impl<T> Deref for WriteGuard<'_, T> { type Target = T; fn deref(&self) -> &T { unsafe { &*self.l.val.get() } } }
impl<T> DerefMut for WriteGuard<'_, T> { fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.l.val.get() } } }
impl<T> Drop for WriteGuard<'_, T> { fn drop(&mut self) { self.l.state.set(0); } }

enum Cow2<'a, T: Clone> { Borrowed(&'a T), Owned(T) }
impl<'a, T: Clone> Cow2<'a, T> {
    fn to_mut(&mut self) -> &mut T {
        if let Cow2::Borrowed(b) = *self { *self = Cow2::Owned(b.clone()); }
        match self { Cow2::Owned(o) => o, Cow2::Borrowed(_) => unreachable!() }
    }
    fn is_owned(&self) -> bool { matches!(self, Cow2::Owned(_)) }
}
impl<T: Clone> Deref for Cow2<'_, T> { type Target = T; fn deref(&self) -> &T { match self { Cow2::Borrowed(b) => b, Cow2::Owned(o) => o } } }

struct Wrapper(Vec<String>);
impl Deref for Wrapper { type Target = Vec<String>; fn deref(&self) -> &Vec<String> { &self.0 } }
impl DerefMut for Wrapper { fn deref_mut(&mut self) -> &mut Vec<String> { &mut self.0 } }
impl fmt::Display for Wrapper { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "[{}]", self.join("|")) } }

fn takes_str(s: &str) -> usize { s.chars().count() }
fn takes_slice(s: &[i32]) -> i32 { s.iter().sum() }

fn main() {
    let b = MyBox::new(String::from("héllo"));
    println!("len {} upper {} takes_str {} starts {}", b.len(), b.to_uppercase(), takes_str(&b), b.starts_with('h'));
    println!("deref count {}", b.derefs());
    let mut mb = MyBox::new(vec![3, 1, 2]);
    mb.push(0);
    mb.sort();
    let total = takes_slice(&mb);
    println!("vec {:?} total {} first {:?} count {}", *mb, total, mb.first(), mb.derefs());
    let nested = MyBox::new(MyBox::new(5i32));
    println!("nested {} abs {}", **nested + 1, nested.abs());

    let lock = Lock::new(vec![1, 2, 3]);
    {
        let r1 = lock.read().unwrap();
        let r2 = lock.read().unwrap();
        println!("readers state {} sum {} write? {}", lock.state.get(), r1.iter().sum::<i32>() + r2.len() as i32, lock.write().is_some());
    }
    println!("after readers state {}", lock.state.get());
    {
        let mut w = lock.write().unwrap();
        w.push(4);
        w.retain(|x| x % 2 == 0);
        println!("writer state {} read? {}", lock.state.get(), lock.read().is_some());
    }
    println!("after writer {:?} state {}", *lock.read().unwrap(), lock.state.get());
    let len = lock.write().map(|mut g| { g.clear(); g.len() });
    println!("write in closure {:?} state {}", len, lock.state.get());

    let base = vec![1, 2, 3];
    let mut c = Cow2::Borrowed(&base);
    println!("cow len {} owned {}", c.len(), c.is_owned());
    c.to_mut().push(4);
    println!("cow {:?} owned {} base {:?}", *c, c.is_owned(), base);

    let mut w = Wrapper(vec!["a".into()]);
    w.push("b".into());
    w[0].push('!');
    w.iter_mut().for_each(|s| s.insert(0, '>'));
    println!("{} len {} contains {}", w, w.len(), w.contains(&">b".to_string()));
    let r: &Vec<String> = &w;
    let s: &[String] = &w;
    println!("coerced {} {}", r.len(), s[1]);
    let bx: Box<dyn Fn(&str) -> usize> = Box::new(takes_str);
    let rc = std::rc::Rc::new(String::from("rc-string"));
    println!("fn via box {} rc deref {}", bx(&rc), takes_str(&rc));
}
