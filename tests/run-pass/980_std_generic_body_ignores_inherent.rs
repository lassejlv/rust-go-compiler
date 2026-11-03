// std's generic bodies (Vec::from_iter, Iterator::sum, ToString, Ord for
// sorting, Debug of containers) call the trait methods of their bounds, never
// an inherent method of the same name on the element/iterator type.
use std::fmt;
use std::rc::Rc;
use std::collections::BTreeMap;
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct B(u8);
impl B {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result { panic!("inherent fmt") }
    fn to_string(&self) -> String { "inherent".into() }
    fn clone(&self) -> B { panic!("inherent clone") }
    fn cmp(&self, _o: &B) -> std::cmp::Ordering { panic!("inherent cmp") }
    fn eq(&self, _o: &B) -> bool { panic!("inherent eq") }
    fn hash(&self) { panic!("inherent hash") }
    fn next(&mut self) -> Option<u8> { panic!("inherent next") }
    fn into_iter(self) -> u8 { panic!("inherent into_iter") }
}
impl Clone for B { fn clone(&self) -> B { B(self.0 + 1) } }
impl fmt::Display for B { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "B{}", self.0) } }
impl fmt::Debug for B { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "dbgB{}", self.0) } }
struct It(u8);
impl It { fn next(&mut self) -> Option<u8> { panic!("inherent next") } }
impl Iterator for It { type Item = u8; fn next(&mut self) -> Option<u8> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0) } } }
fn main() {
    let b = B(1);
    println!("{} {} {:?}", &b, Box::new(B(2)), Rc::new(B(3)));
    println!("{}", (&b).to_string());
    println!("{:?}", vec![B(4)].clone());
    println!("{:?}", Some(&b).cloned());
    let mut v = vec![B(3), B(1), B(2)];
    v.sort();
    println!("{:?} {}", v, v.contains(&B(2)));
    let mut m = BTreeMap::new(); m.insert(B(5), 1); m.insert(B(4), 2);
    println!("{:?}", m);
    let mut hs = std::collections::HashSet::new(); hs.insert(B(9));
    println!("{}", hs.contains(&B(9)));
    println!("{:?}", It(3).collect::<Vec<_>>());
    println!("{:?}", It(3).map(|x| x * 2).sum::<u8>());
    println!("{:?}", Some(B(7)).iter().map(|x| x.to_string()).collect::<Vec<_>>());
    println!("{}", [B(1), B(2)].iter().max().unwrap());
}
