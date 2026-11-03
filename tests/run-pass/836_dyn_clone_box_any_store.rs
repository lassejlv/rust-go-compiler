use std::any::Any;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;
trait Shape: Any + Debug { fn area(&self) -> f64; fn scale(&mut self, k: f64); fn clone_box(&self) -> Box<dyn Shape>; fn as_any(&self) -> &dyn Any; fn with<F: FnOnce(&Self) -> R, R>(&self, f: F) -> R where Self: Sized { f(self) } }
#[derive(Debug, Clone, PartialEq)] struct Sq(f64);
#[derive(Debug, Clone, PartialEq)] struct Ci(f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } fn scale(&mut self, k: f64) { self.0 *= k } fn clone_box(&self) -> Box<dyn Shape> { Box::new(self.clone()) } fn as_any(&self) -> &dyn Any { self } }
impl Shape for Ci { fn area(&self) -> f64 { 3.0 * self.0 * self.0 } fn scale(&mut self, k: f64) { self.0 *= k } fn clone_box(&self) -> Box<dyn Shape> { Box::new(self.clone()) } fn as_any(&self) -> &dyn Any { self } }
impl Clone for Box<dyn Shape> { fn clone(&self) -> Self { self.clone_box() } }
impl dyn Shape { fn is_big(&self) -> bool { self.area() > 10.0 } fn downcast_sq(&self) -> Option<&Sq> { self.as_any().downcast_ref() } }
impl PartialEq for dyn Shape { fn eq(&self, o: &dyn Shape) -> bool { self.area() == o.area() } }
impl Display for dyn Shape { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "<{:.1}>", self.area()) } }
trait Store { type Key: Ord + Debug; type Val; fn put(&mut self, k: Self::Key, v: Self::Val); fn dump(&self) -> Vec<(&Self::Key, &Self::Val)>; }
#[derive(Default)] struct VecStore<K, V> { v: Vec<(K, V)> }
impl<K: Ord + Debug, V> Store for VecStore<K, V> { type Key = K; type Val = V; fn put(&mut self, k: K, v: V) { match self.v.binary_search_by(|p| p.0.cmp(&k)) { Ok(i) => self.v[i].1 = v, Err(i) => self.v.insert(i, (k, v)) } } fn dump(&self) -> Vec<(&K, &V)> { self.v.iter().map(|(k, v)| (k, v)).collect() } }
fn fill<S: Store<Key = String, Val = usize> + ?Sized>(s: &mut S, words: &str) { for w in words.split(' ') { s.put(w.to_string(), w.len()); } }
fn from_pairs<I, K, V>(it: I) -> VecStore<K, V> where I: IntoIterator<Item = (K, V)>, K: Ord + Debug { let mut s = VecStore { v: vec![] }; for (k, v) in it { s.put(k, v); } s }
trait Named { fn name(&self) -> String; }
impl<T: ?Sized + Named> Named for Box<T> { fn name(&self) -> String { format!("box[{}]", (**self).name()) } }
impl<T: Named> Named for Option<T> { fn name(&self) -> String { self.as_ref().map_or("none".into(), |t| t.name()) } }
impl Named for str { fn name(&self) -> String { self.to_string() } }
impl Named for i32 { fn name(&self) -> String { format!("i{}", self) } }
struct Noisy<T: Debug>(T);
impl<T: Debug> Drop for Noisy<T> { fn drop(&mut self) { println!("drop {:?}", self.0); } }
fn main() {
    let mut shapes: Vec<Box<dyn Shape>> = vec![Box::new(Sq(2.0)), Box::new(Ci(2.0)), Box::new(Sq(4.0))];
    let copy = shapes.clone();
    for s in shapes.iter_mut() { s.scale(0.5); }
    for (a, b) in shapes.iter().zip(copy.iter()) { println!("{} {} {:?} big={} eq={} sq={:?}", a, b, a, b.is_big(), **a == **b, b.downcast_sq()); }
    println!("{}", Sq(3.0).with(|s| s.area() + 1.0));
    let any_rc: Rc<dyn Any> = Rc::new(Ci(1.0));
    println!("{:?} {:?}", any_rc.clone().downcast::<Sq>().is_err(), any_rc.downcast::<Ci>().map(|c| c.0));
    let b: Box<dyn Any> = Box::new(5u8);
    println!("{:?}", b.downcast::<u8>().map(|x| *x + 1).ok());
    let mut st: VecStore<String, usize> = VecStore::default();
    fill(&mut st, "pear fig apple fig kiwi");
    println!("{:?}", st.dump());
    let dynst: &mut dyn Store<Key = String, Val = usize> = &mut st;
    fill(dynst, "zz a");
    println!("{:?}", dynst.dump());
    println!("{:?}", from_pairs(vec![(3, 'c'), (1, 'a'), (3, 'C')]).dump());
    let bx: Box<str> = "hey".into();
    println!("{} {} {} {}", bx.name(), Box::new(Box::new(5)).name(), Some(Box::new(7)).name(), None::<i32>.name());
    let dn: Box<dyn Named> = Box::new(3);
    println!("{}", dn.name());
    {
        let _a = Noisy("a");
        let v = vec![Noisy(1), Noisy(2)];
        let _moved = v.into_iter().nth(1);
        println!("end scope");
    }
    let t = (Noisy('x'), Noisy('y'));
    let (_, y) = t;
    println!("after destructure");
    drop(y);
    println!("end main");
}
