// References created by dereferencing raw pointers stored behind `&self`/`&mut self` are not
// tied to that borrow (their lifetime is unbounded), so they may be returned with the
// struct's lifetime parameter or another chosen lifetime.
use std::marker::PhantomData;

struct Slot {
    v: i32,
}

struct Cursor<'a> {
    cur: *mut Slot,
    _m: PhantomData<&'a mut Slot>,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a mut [Slot]) -> Self {
        Cursor { cur: s.as_mut_ptr(), _m: PhantomData }
    }
    fn get(&mut self) -> &'a mut Slot {
        unsafe {
            let h = &mut *self.cur;
            h
        }
    }
    fn peek(&self) -> &'a Slot {
        unsafe { &*self.cur }
    }
    fn advance(&mut self) {
        self.cur = unsafe { self.cur.add(1) };
    }
}

struct Holder {
    p: *const String,
}
fn leak_ref<'x>(h: &Holder) -> &'x String {
    unsafe { &*h.p }
}

struct Table {
    rows: *mut Vec<u32>,
}
impl Table {
    fn row_mut<'b>(&self, i: usize) -> &'b mut u32 {
        unsafe { &mut (&mut *self.rows)[i] }
    }
}

fn main() {
    let mut slots = [Slot { v: 1 }, Slot { v: 2 }, Slot { v: 3 }];
    {
        let mut c = Cursor::new(&mut slots);
        let a = c.get();
        c.advance();
        let b = c.peek();
        a.v += b.v * 10;
        c.advance();
        let last = c.get();
        last.v = 99;
    }
    println!("{:?}", slots.iter().map(|s| s.v).collect::<Vec<_>>());

    let s = String::from("held");
    let r = {
        let h = Holder { p: &s };
        leak_ref(&h)
    };
    println!("{}", r);

    let mut data = vec![5u32, 6, 7];
    let t = Table { rows: &mut data };
    let x = t.row_mut(0);
    let y = t.row_mut(2);
    *x += 100;
    *y += 200;
    drop(t);
    println!("{:?}", data);
}
