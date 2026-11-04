// Debug impls of std iterator / entry / cell types that were missing
// (deriving Debug on a struct holding them failed to compile).
use std::cell::LazyCell;
use std::collections::*;
use std::process::ExitCode;

#[derive(Debug)]
struct Holder<'a> {
    vals: btree_map::ValuesMut<'a, i32, char>,
    it: std::option::IntoIter<u8>,
}

fn main() {
    let v = [1, 2, 3];
    println!("{:?}", v.iter().skip_while(|x| **x < 2));
    println!("{:?}", v.iter().take_while(|x| **x < 2));
    println!("{:?}", v.iter().map_while(|x| Some(x)));
    println!("{:?}", v.iter().scan(0, |s, x| { *s += x; Some(*s) }));
    println!("{:?}", v.iter().inspect(|_| {}));
    println!("{:?}", v.iter().flat_map(|x| vec![*x]));
    println!("{:?}", vec![vec![1]].into_iter().flatten());
    println!("{:?}", std::iter::once_with(|| 1));
    println!("{:?}", std::iter::repeat_with(|| 1));
    println!("{:?}", std::iter::once(5));
    println!("{:?}", "ab".encode_utf16());
    let mut it = v.iter().skip_while(|x| **x < 2);
    it.next();
    println!("{:?}", it);

    let mut bm = BTreeMap::from([(1, 'a'), (2, 'b')]);
    println!("{:?}", bm.values_mut());
    println!("{:?}", bm.clone().into_keys());
    println!("{:?}", bm.clone().into_values());
    println!("{:?}", bm.clone().into_iter());
    println!("{:?}", bm.iter_mut());
    println!("{:?}", bm.range_mut(2..));
    println!("{:?}", bm.entry(1));
    println!("{:?}", bm.entry(9));
    if let btree_map::Entry::Occupied(o) = bm.entry(1) {
        println!("{:?}", o);
    }
    if let btree_map::Entry::Vacant(v) = bm.entry(9) {
        println!("{:?}", v);
    }
    let bs = BTreeSet::from([1, 2]);
    println!("{:?}", bs.clone().into_iter());
    println!("{:?}", bs.range(2..));

    let mut hm = HashMap::from([(1, 'a')]);
    println!("{:?}", hm.values_mut());
    println!("{:?}", hm.iter_mut());
    println!("{:?}", hm.clone().into_keys());
    println!("{:?}", hm.clone().into_values());
    println!("{:?}", hm.clone().into_iter());
    println!("{:?}", hm.drain());
    let mut hs = HashSet::from([7]);
    println!("{:?}", hs.clone().into_iter());
    println!("{:?}", hs.drain());

    let mut ll = LinkedList::from([1, 2, 3]);
    println!("{:?}", ll.iter());
    let mut li = ll.iter();
    li.next();
    println!("{:?}", li);
    println!("{:?}", ll.iter_mut());
    println!("{:?}", ll.clone().into_iter());
    let mut dq = VecDeque::from([4, 5]);
    println!("{:?}", dq.iter_mut());

    let mut o = Some(1);
    println!("{:?}", o.iter());
    println!("{:?}", o.iter_mut());
    println!("{:?}", o.into_iter());
    let mut r: Result<i32, ()> = Ok(1);
    println!("{:?}", r.iter());
    println!("{:?}", r.iter_mut());
    println!("{:?}", r.into_iter());

    let mut bm2 = BTreeMap::from([(3, 'c')]);
    let h = Holder { vals: bm2.values_mut(), it: Some(9u8).into_iter() };
    println!("{:?}", h);
    println!("{:#?}", h);

    let lc = LazyCell::new(|| 1);
    println!("{:?}", lc);
    println!("{}", *lc);
    println!("{:?}", lc);
    let d: LazyCell<Vec<i32>> = LazyCell::default();
    println!("{:?} {:?}", d.len(), d);

    println!("{:?} {:?} {:?}", ExitCode::SUCCESS, ExitCode::FAILURE, ExitCode::from(42));
}
