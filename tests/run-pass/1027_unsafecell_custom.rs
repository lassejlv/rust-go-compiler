use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

struct MyCell<T> { v: UnsafeCell<T> }
impl<T: Copy> MyCell<T> {
    fn new(v: T) -> Self { MyCell { v: UnsafeCell::new(v) } }
    fn get(&self) -> T { unsafe { *self.v.get() } }
    fn set(&self, v: T) { unsafe { *self.v.get() = v; } }
}

struct MyRefCell<T> { flag: MyCell<isize>, v: UnsafeCell<T> }
struct Ref<'a, T> { c: &'a MyRefCell<T> }
struct RefMut<'a, T> { c: &'a MyRefCell<T>, _p: PhantomData<&'a mut T> }

impl<T> MyRefCell<T> {
    fn new(v: T) -> Self { MyRefCell { flag: MyCell::new(0), v: UnsafeCell::new(v) } }
    fn try_borrow(&self) -> Option<Ref<'_, T>> {
        if self.flag.get() < 0 { return None; }
        self.flag.set(self.flag.get() + 1);
        Some(Ref { c: self })
    }
    fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        if self.flag.get() != 0 { return None; }
        self.flag.set(-1);
        Some(RefMut { c: self, _p: PhantomData })
    }
}
impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { unsafe { &*self.c.v.get() } }
}
impl<'a, T> Drop for Ref<'a, T> {
    fn drop(&mut self) { self.c.flag.set(self.c.flag.get() - 1); }
}
impl<'a, T> Deref for RefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { unsafe { &*self.c.v.get() } }
}
impl<'a, T> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.c.v.get() } }
}
impl<'a, T> Drop for RefMut<'a, T> {
    fn drop(&mut self) { self.c.flag.set(0); }
}

fn main() {
    let c = MyCell::new(5);
    let r1 = &c;
    let r2 = &c;
    r1.set(10);
    println!("{}", r2.get());
    let rc = MyRefCell::new(vec![1, 2]);
    {
        let a = rc.try_borrow().unwrap();
        let b = rc.try_borrow().unwrap();
        println!("two borrows {:?} {:?} flag={}", *a, *b, rc.flag.get());
        println!("mut while shared: {}", rc.try_borrow_mut().is_none());
    }
    println!("flag after {}", rc.flag.get());
    {
        let mut m = rc.try_borrow_mut().unwrap();
        m.push(3);
        println!("shared while mut: {} flag={}", rc.try_borrow().is_none(), rc.flag.get());
    }
    println!("{:?} flag={}", *rc.try_borrow().unwrap(), rc.flag.get());
    let mut uc = UnsafeCell::new(1);
    *uc.get_mut() += 1;
    println!("{}", uc.into_inner());
    let cell = std::cell::Cell::new(3);
    let mut backing = [1, 2, 3];
    let sl: &[std::cell::Cell<i32>] = std::cell::Cell::from_mut(&mut backing[..]).as_slice_of_cells();
    sl[0].set(sl[1].get() + sl[2].get());
    println!("{:?}", sl.iter().map(|c| c.get()).collect::<Vec<_>>());
    println!("{}", cell.replace(4) + cell.get());
}

