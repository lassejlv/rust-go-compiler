// static mut via &raw mut / &raw const, UnsafeCell, Cell::as_ptr, Rc::into_raw/from_raw,
// ptr::drop_in_place, black_box.
use std::cell::{Cell, UnsafeCell};
use std::hint::black_box;
use std::ptr;
use std::rc::Rc;

static mut COUNTER: u64 = 0;
static mut TABLE: [u8; 8] = [0; 8];

fn bump(by: u64) -> u64 {
    unsafe {
        let p = &raw mut COUNTER;
        *p += by;
        *p
    }
}

struct MyCell<T> {
    v: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for MyCell<T> {}
impl<T: Copy> MyCell<T> {
    const fn new(v: T) -> Self {
        MyCell { v: UnsafeCell::new(v) }
    }
    fn get(&self) -> T {
        unsafe { *self.v.get() }
    }
    fn set(&self, v: T) {
        unsafe { *self.v.get() = v }
    }
}

static GLOBAL: MyCell<i32> = MyCell::new(-1);

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    bump(5);
    bump(7);
    println!("counter {}", unsafe { *(&raw const COUNTER) });
    unsafe {
        let t = &raw mut TABLE;
        for i in 0..8 {
            (*t)[i] = (i * i) as u8;
        }
        let tp = t as *mut u8;
        *tp.add(7) = 255;
        println!("{:?}", *(&raw const TABLE));
    }
    GLOBAL.set(41);
    GLOBAL.set(GLOBAL.get() + 1);
    println!("global {}", GLOBAL.get());

    let c = Cell::new(10);
    let p = c.as_ptr();
    unsafe { *p += 5 };
    println!("cell {}", c.get());
    let mut uc = UnsafeCell::new(vec![1]);
    uc.get_mut().push(2);
    unsafe { (*uc.get()).push(3) };
    println!("{:?}", uc.into_inner());
    let mut arr3 = [1, 2, 3];
    let slice_cells = Cell::from_mut(&mut arr3[..]).as_slice_of_cells();
    slice_cells[0].set(100);
    println!("{}", slice_cells.iter().map(|c| c.get()).sum::<i32>());

    let rc = Rc::new(String::from("rc string"));
    let rc2 = rc.clone();
    let raw = Rc::into_raw(rc);
    unsafe {
        println!("{} count {}", *raw, Rc::strong_count(&rc2));
        Rc::increment_strong_count(raw);
        println!("count {}", Rc::strong_count(&rc2));
        Rc::decrement_strong_count(raw);
        let back = Rc::from_raw(raw);
        println!("back {} count {}", back, Rc::strong_count(&back));
    }
    println!("count after {}", Rc::strong_count(&rc2));

    let mut slots = std::mem::ManuallyDrop::new([D("s0"), D("s1"), D("s2")]);
    unsafe {
        ptr::drop_in_place(&mut slots[1]);
        ptr::write(&mut slots[1], D("s1-new"));
        ptr::drop_in_place(&mut *slots as *mut [D; 3]);
    }
    println!("after drop_in_place");
    let b = Box::new(D("boxed"));
    let raw = Box::into_raw(b);
    unsafe {
        ptr::drop_in_place(raw);
        std::alloc::dealloc(raw as *mut u8, std::alloc::Layout::new::<D>());
    }
    let x = black_box(21) * 2;
    println!("black_box {}", x);
    let v = black_box(vec![1, 2, 3]);
    println!("{}", v.len());
}
