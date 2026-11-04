// Breadth test of Rc/Arc/Weak, Cell/RefCell/OnceCell, Cow and std::mem helpers.
use std::borrow::Cow;
use std::cell::{Cell, OnceCell, Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};
use std::sync::Arc;

fn normalize(s: &str) -> Cow<'_, str> {
    if s.contains(char::is_uppercase) || s.contains(' ') {
        Cow::Owned(s.to_lowercase().replace(' ', "_"))
    } else {
        Cow::Borrowed(s)
    }
}

fn abs_all(v: &mut Cow<'_, [i32]>) {
    for i in 0..v.len() {
        if v[i] < 0 {
            v.to_mut()[i] = -v[i];
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Big { data: Vec<u8>, tag: String }

fn main() {
    // Cow
    for s in ["plain", "Has Caps", "with space"] {
        let c = normalize(s);
        println!("{:<12} -> {:<12} borrowed={}", s, c, matches!(c, Cow::Borrowed(_)));
    }
    let src = [1, -2, 3];
    let mut c1: Cow<[i32]> = Cow::Borrowed(&src[..]);
    abs_all(&mut c1);
    let pos = [1, 2];
    let mut c2: Cow<[i32]> = Cow::Borrowed(&pos[..]);
    abs_all(&mut c2);
    println!("cow slices {:?} owned={} {:?} owned={}", c1, matches!(c1, Cow::Owned(_)), c2, matches!(c2, Cow::Owned(_)));
    let owned: String = normalize("A B").into_owned();
    let mut cs: Cow<str> = Cow::from("abc");
    cs += "def";
    println!("into_owned {} add_assign {} len {}", owned, cs, cs.len());
    let cow_cmp = Cow::Borrowed("x") == Cow::<str>::Owned("x".to_string());
    println!("cow eq {}", cow_cmp);

    // Rc / Weak
    let a = Rc::new(Big { data: vec![1, 2, 3], tag: "a".into() });
    let b = Rc::clone(&a);
    let w: Weak<Big> = Rc::downgrade(&a);
    println!("strong {} weak {} ptr_eq {} upgrade {:?}", Rc::strong_count(&a), Rc::weak_count(&a), Rc::ptr_eq(&a, &b), w.upgrade().map(|x| x.tag.clone()));
    let mut c = Rc::clone(&a);
    Rc::make_mut(&mut c).tag.push_str("-cow");
    println!("make_mut cloned: a={} c={} strong(a)={} strong(c)={}", a.tag, c.tag, Rc::strong_count(&a), Rc::strong_count(&c));
    let mut solo = Rc::new(5);
    *Rc::get_mut(&mut solo).unwrap() += 1;
    let shared = Rc::clone(&solo);
    println!("get_mut unique ok, now shared -> {:?}", Rc::get_mut(&mut solo).is_some());
    drop(shared);
    println!("try_unwrap {:?} {:?}", Rc::try_unwrap(solo), Rc::try_unwrap(Rc::clone(&a)).map_err(|rc| Rc::strong_count(&rc)));
    println!("into_inner {:?}", Rc::into_inner(Rc::new("only")));
    drop(b);
    drop(a);
    println!("weak after drop: {:?} strong {}", w.upgrade().is_none(), w.strong_count());
    let empty: Weak<i32> = Weak::new();
    println!("Weak::new upgrade {:?}", empty.upgrade());
    let rc_str: Rc<str> = Rc::from("shared str");
    let rc_slice: Rc<[u8]> = Rc::from(vec![9u8, 8]);
    println!("{} {:?} {}", rc_str, rc_slice, rc_str.len());
    let arc = Arc::new(vec![1, 2]);
    let arc2 = Arc::clone(&arc);
    println!("arc strong {} eq {} unwrap_or_clone {:?}", Arc::strong_count(&arc), arc == arc2, Arc::unwrap_or_clone(arc2));

    // Cell
    let cell = Cell::new(10);
    let old = cell.replace(20);
    cell.set(cell.get() + 1);
    println!("cell replace {} get {} take {} after {}", old, cell.get(), cell.take(), cell.get());
    let c1 = Cell::new(1);
    let c2 = Cell::new(2);
    c1.swap(&c2);
    println!("swap {} {} update-ish {}", c1.get(), c2.get(), { c1.set(c1.get() * 7); c1.into_inner() });
    let cells: Vec<Cell<u8>> = (0..3).map(Cell::new).collect();
    cells.iter().for_each(|c| c.set(c.get() + 100));
    println!("cells {:?}", cells.iter().map(Cell::get).collect::<Vec<_>>());
    let mut arr = [1, 2, 3];
    let slice_cells = Cell::from_mut(&mut arr[..]).as_slice_of_cells();
    slice_cells[0].set(slice_cells[2].get());
    println!("as_slice_of_cells {:?}", arr);

    // RefCell
    let rc = RefCell::new(vec![1, 2, 3]);
    {
        let r1 = rc.borrow();
        let r2 = rc.borrow();
        println!("two borrows {} {} try_borrow_mut err {}", r1.len(), r2[0], rc.try_borrow_mut().is_err());
    }
    rc.borrow_mut().push(4);
    let old = rc.replace(vec![9]);
    println!("replace old {:?} new {:?}", old, rc.borrow());
    rc.replace_with(|v| v.iter().map(|x| x * 2).collect());
    println!("replace_with {:?}", rc.borrow());
    let tup = RefCell::new((String::from("left"), 5u32));
    {
        let name: Ref<String> = Ref::map(tup.borrow(), |t| &t.0);
        println!("Ref::map {} still can borrow: {}", *name, tup.try_borrow().is_ok());
    }
    {
        let (mut s, mut n) = RefMut::map_split(tup.borrow_mut(), |t| (&mut t.0, &mut t.1));
        s.push_str("++");
        *n += 1;
    }
    println!("map_split {:?}", tup.borrow());
    let filtered = Ref::filter_map(rc.borrow(), |v| v.first());
    println!("filter_map {:?}", filtered.map(|r| *r));
    let taken = rc.take();
    println!("take {:?} left {:?} into_inner {:?}", taken, rc.borrow(), RefCell::new(3).into_inner());
    let shared = Rc::new(RefCell::new(0));
    let handles: Vec<Rc<RefCell<i32>>> = (0..4).map(|_| Rc::clone(&shared)).collect();
    for (i, h) in handles.iter().enumerate() {
        *h.borrow_mut() += i as i32 * 10;
    }
    println!("shared counter {} strong {}", shared.borrow(), Rc::strong_count(&shared));

    // OnceCell
    let once: OnceCell<String> = OnceCell::new();
    println!("once get {:?}", once.get());
    let v1 = once.get_or_init(|| "first".to_string()).clone();
    let v2 = once.get_or_init(|| "second".to_string()).clone();
    println!("once {} {} set err {:?}", v1, v2, once.set("third".into()));
    let mut once2: OnceCell<u32> = OnceCell::from(7);
    println!("take {:?} {:?}", once2.take(), once2.get());

    // mem
    let mut x = String::from("x");
    let mut y = String::from("y");
    std::mem::swap(&mut x, &mut y);
    let prev = std::mem::replace(&mut x, "new".into());
    let took = std::mem::take(&mut y);
    println!("swap/replace/take: x={} prev={} took={} y={:?}", x, prev, took, y);
    let mut big = Big { data: vec![7; 3], tag: "t".into() };
    let data = std::mem::take(&mut big.data);
    println!("take field {:?} -> {:?}", data, big);
    println!("size_of: {} {} {} {} {}", std::mem::size_of::<Option<Box<u8>>>(), std::mem::size_of::<Option<u32>>(), std::mem::size_of::<(u8, u32)>(), std::mem::size_of::<[u16; 5]>(), std::mem::size_of::<Rc<u8>>());
    println!("align_of {} {} discriminant eq {}", std::mem::align_of::<u64>(), std::mem::align_of::<u8>(), std::mem::discriminant(&Some(1)) == std::mem::discriminant(&Some(9)));
    let boxed = Box::new(Big::default());
    let leaked: &'static mut Big = Box::leak(boxed);
    leaked.tag.push('L');
    println!("leaked {:?}", leaked.tag);
    let bx: Box<dyn Fn(i32) -> i32> = Box::new(|v| v + 1);
    let rcf: Rc<dyn Fn(i32) -> i32> = Rc::new(move |v| bx(v) * 2);
    println!("rc dyn fn {}", rcf(4));
}
