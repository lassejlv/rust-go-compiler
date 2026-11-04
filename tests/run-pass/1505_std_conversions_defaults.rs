// Conversions and trait impls that were missing: NonZero<A> -> NonZero<B>
// (From / TryFrom), Box/Rc/Arc/Cow<CStr>, Rc/Arc<OsStr>, CString::from_raw,
// Default for iterators, Clone for some iterators, binary_heap::{Iter, IntoIter, Drain}.
use std::borrow::Cow;
use std::collections::*;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::num::*;
use std::rc::Rc;
use std::sync::Arc;

fn count_default<T: Default + Iterator>() -> usize {
    T::default().count()
}

fn first_of<I: Iterator + Clone>(it: &I) -> Option<I::Item> {
    it.clone().next()
}

fn main() {
    // NonZero conversions
    let a: NonZeroU64 = NonZeroU32::new(5).unwrap().into();
    let b: NonZeroI16 = NonZeroU8::new(200).unwrap().into();
    let c = NonZeroU8::try_from(NonZeroU32::new(300).unwrap());
    let d = NonZeroU8::try_from(NonZeroI64::new(-1).unwrap());
    let e = NonZeroI8::try_from(NonZeroU16::new(100).unwrap());
    let f: Result<NonZeroU64, _> = NonZeroU32::new(5).unwrap().try_into();
    let g = NonZeroUsize::try_from(NonZeroU64::new(7).unwrap());
    let i: NonZeroIsize = NonZeroI16::new(-3).unwrap().into();
    let j = NonZeroU128::from(NonZeroU64::new(2).unwrap());
    println!("{} {} {:?} {:?} {:?} {:?} {:?} {} {}", a, b, c, d, e, f, g, i, j);
    println!("{}", c.unwrap_err());

    // CStr / CString
    let cs = CString::new("hello").unwrap();
    let bx: Box<CStr> = cs.clone().into_boxed_c_str();
    println!("{:?} {:?}", bx, bx.clone().into_c_string());
    let b2: Box<CStr> = cs.as_c_str().into();
    let r: Rc<CStr> = cs.as_c_str().into();
    let ar: Arc<CStr> = cs.clone().into();
    let back: CString = b2.into();
    println!("{:?} {:?} {:?}", r, ar, back);
    let raw = cs.clone().into_raw();
    let again = unsafe { CString::from_raw(raw) };
    let unchecked = unsafe { CString::from_vec_unchecked(b"abc".to_vec()) };
    println!("{:?} {:?} {:?}", again, unchecked, unchecked.as_bytes_with_nul());
    let cow: Cow<CStr> = Cow::Borrowed(cs.as_c_str());
    let owned: CString = cow.into();
    let cow2: Cow<CStr> = (&owned).into();
    let cow3: Cow<CStr> = owned.clone().into();
    println!("{:?} {:?} {}", cow2, cow3, matches!(cow3, Cow::Owned(_)));
    let empty: &CStr = Default::default();
    println!("{:?} {:?} {:?}", empty, &cs[..], <Box<CStr>>::default());
    let nz: Vec<NonZeroU8> = b"nz".iter().map(|b| NonZeroU8::new(*b).unwrap()).collect();
    println!("{:?}", CString::from(nz));

    // OsStr
    let o: Rc<OsStr> = OsStr::new("o").into();
    let o2: Arc<OsStr> = OsString::from("o2").into();
    let o3: Arc<OsStr> = OsStr::new("o3").into();
    println!("{:?} {:?} {:?} {}", o, o2, o3, o3.len());

    // Default iterators
    let n = count_default::<btree_map::Iter<i32, i32>>()
        + count_default::<btree_map::IterMut<i32, i32>>()
        + count_default::<btree_map::Range<i32, i32>>()
        + count_default::<btree_map::Keys<i32, i32>>()
        + count_default::<btree_map::IntoValues<i32, i32>>()
        + count_default::<btree_set::Iter<i32>>()
        + count_default::<btree_set::IntoIter<i32>>()
        + count_default::<hash_map::Iter<i32, i32>>()
        + count_default::<hash_map::IntoKeys<i32, i32>>()
        + count_default::<hash_set::Iter<i32>>()
        + count_default::<vec_deque::Iter<i32>>()
        + count_default::<linked_list::Iter<i32>>()
        + count_default::<linked_list::IntoIter<i32>>()
        + count_default::<std::iter::Rev<std::slice::Iter<i32>>>()
        + count_default::<std::iter::Chain<std::vec::IntoIter<i32>, std::vec::IntoIter<i32>>>()
        + count_default::<std::iter::Copied<std::slice::Iter<i32>>>()
        + count_default::<std::iter::Cloned<std::slice::Iter<i32>>>()
        + count_default::<std::iter::Enumerate<std::slice::Iter<i32>>>()
        + count_default::<std::iter::Fuse<std::vec::IntoIter<i32>>>()
        + count_default::<std::iter::Flatten<std::vec::IntoIter<Vec<i32>>>>();
    println!("{}", n);

    // Clone of iterators
    let v = [1, 1, 2, 3, 3];
    println!("{:?}", first_of(&v.chunk_by(|a, b| a == b)));
    println!("{:?}", first_of(&"a1b2".rmatches(char::is_numeric)));
    println!("{:?}", first_of(&"a1b2".rmatch_indices(char::is_numeric)));
    let res: Result<i32, ()> = Ok(4);
    println!("{:?} {:?}", first_of(&res.iter()), first_of(&res.into_iter()));
    println!("{:?}", first_of(&LinkedList::from([9, 8]).into_iter()));

    // binary_heap iterator types
    let mut h = BinaryHeap::from(vec![3, 1, 2]);
    let it: binary_heap::Iter<i32> = h.iter();
    println!("{}", it.count());
    let d: binary_heap::Drain<i32> = h.drain();
    println!("{}", d.count());
    let ii: binary_heap::IntoIter<i32> = BinaryHeap::from(vec![5]).into_iter();
    println!("{:?}", ii.collect::<Vec<_>>());
}
