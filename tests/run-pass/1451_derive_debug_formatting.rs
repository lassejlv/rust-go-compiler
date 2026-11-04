// Debug formatting details of derived impls
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::marker::PhantomData;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Debug)] struct Empty {}
#[derive(Debug)] struct EmptyTuple();
#[derive(Debug)] struct UnitS;
#[derive(Debug)] struct Ref<'a, T: ?Sized> { r: &'a T, p: PhantomData<T> }
#[derive(Debug)] enum E { A, B(), C {}, D(i32, &'static str), F { x: f64, y: Option<Box<E>> } }
#[derive(Debug)] struct Nest { v: Vec<(u8, char)>, m: BTreeMap<String, Vec<i32>>, s: BTreeSet<i8>, q: VecDeque<bool> }
#[derive(Debug)] struct Cells { c: Cell<i32>, r: RefCell<Vec<u8>>, rc: Rc<str> }
#[derive(Debug)] struct Raw { r#type: u8, r#fn: () }

fn main() {
    println!("{:?} {:?} {:?}", Empty {}, EmptyTuple(), UnitS);
    println!("{:#?} {:#?} {:#?}", Empty {}, EmptyTuple(), UnitS);
    let x = 5;
    println!("{:?}", Ref { r: &x, p: PhantomData });
    println!("{:?}", Ref::<str> { r: "s", p: PhantomData });
    let es = [E::A, E::B(), E::C {}, E::D(-1, "q\"uote"), E::F { x: 1.0, y: Some(Box::new(E::A)) }];
    println!("{:?}", es);
    println!("{:#?}", es);
    let mut m = BTreeMap::new();
    m.insert("k1".to_string(), vec![1, 2]);
    m.insert("k0".to_string(), vec![]);
    let n = Nest { v: vec![(1, 'a'), (2, '\n')], m, s: [3, -1].into_iter().collect(), q: VecDeque::from([true]) };
    println!("{:?}", n);
    println!("{:#?}", n);
    let c = Cells { c: Cell::new(3), r: RefCell::new(vec![9]), rc: Rc::from("rc") };
    println!("{:?}", c);
    let _b = c.r.borrow_mut();
    println!("{:?}", c);
    println!("{:?}", Raw { r#type: 1, r#fn: () });
    println!("{:?} {:?} {:?} {:?} {:?}", 1.0f64, 1e20f64, -0.0f64, f64::NAN, 1.5e-7f64);
    println!("{:?} {:?} {:?}", 0.1f32 + 0.2f32, 1e16f64, 123456789.0f32);
    println!("{:?} {:?} {:?}", "tab\t\u{1b}é\u{7f}\0", '\'', '"');
    println!("{:5?}|{:<8?}|{:>+6.1?}", 3, "ab", 2.25);
    println!("{:#?}", (1,));
    println!("{:#?}", Some(Some(())));
    println!("{:?}", (Ok::<i32, ()>(1), Err::<(), _>("e")));
    println!("{:#x?} {:#X?} {:x?}", [255, 16], 255u8, vec![10, 11]);
    println!("{:08.3?} {:e}", -1.5f64, 1500f32);
}
