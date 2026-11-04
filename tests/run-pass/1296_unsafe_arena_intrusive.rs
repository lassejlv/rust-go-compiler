// A typed arena with raw chunks, an intrusive singly linked stack through *mut, a
// small-string-optimized union type, and a manual vtable (struct of fn pointers).
use std::alloc::{alloc, dealloc, Layout};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ptr;

struct Arena<T> {
    chunks: RefCell<Vec<(*mut T, usize)>>,
    used: Cell<usize>,
    chunk_cap: usize,
}

impl<T> Arena<T> {
    fn new(chunk_cap: usize) -> Self {
        Arena { chunks: RefCell::new(Vec::new()), used: Cell::new(chunk_cap), chunk_cap }
    }
    fn alloc(&self, v: T) -> &mut T {
        if self.used.get() == self.chunk_cap {
            let p = unsafe { alloc(Layout::array::<T>(self.chunk_cap).unwrap()) as *mut T };
            self.chunks.borrow_mut().push((p, 0));
            self.used.set(0);
        }
        let mut chunks = self.chunks.borrow_mut();
        let last = chunks.last_mut().unwrap();
        let slot = unsafe { last.0.add(self.used.get()) };
        last.1 += 1;
        self.used.set(self.used.get() + 1);
        unsafe {
            slot.write(v);
            &mut *slot
        }
    }
}
impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        for &(p, n) in self.chunks.borrow().iter() {
            unsafe {
                for i in 0..n {
                    ptr::drop_in_place(p.add(i));
                }
                dealloc(p as *mut u8, Layout::array::<T>(self.chunk_cap).unwrap());
            }
        }
    }
}

struct Named {
    name: String,
    next: *mut Named,
}
impl Drop for Named {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

struct Stack<'a> {
    head: *mut Named,
    _m: PhantomData<&'a mut Named>,
}
impl<'a> Stack<'a> {
    fn push(&mut self, n: &'a mut Named) {
        n.next = self.head;
        self.head = n;
    }
    fn pop(&mut self) -> Option<&'a mut Named> {
        if self.head.is_null() {
            return None;
        }
        unsafe {
            let h = &mut *self.head;
            self.head = h.next;
            h.next = ptr::null_mut();
            Some(h)
        }
    }
}

const INLINE: usize = 15;
#[repr(C)]
union SsoData {
    inline: [u8; INLINE],
    heap: ManuallyDrop<Box<[u8]>>,
}
struct Sso {
    len: u8, // 255 => heap
    data: SsoData,
}
impl Sso {
    fn new(s: &str) -> Sso {
        if s.len() <= INLINE {
            let mut buf = [0u8; INLINE];
            buf[..s.len()].copy_from_slice(s.as_bytes());
            Sso { len: s.len() as u8, data: SsoData { inline: buf } }
        } else {
            Sso { len: 255, data: SsoData { heap: ManuallyDrop::new(s.as_bytes().into()) } }
        }
    }
    fn as_str(&self) -> &str {
        unsafe {
            if self.len == 255 {
                std::str::from_utf8_unchecked(&self.data.heap)
            } else {
                std::str::from_utf8_unchecked(&self.data.inline[..self.len as usize])
            }
        }
    }
    fn is_inline(&self) -> bool {
        self.len != 255
    }
}
impl Drop for Sso {
    fn drop(&mut self) {
        if self.len == 255 {
            unsafe { ManuallyDrop::drop(&mut self.data.heap) }
        }
    }
}

struct VTable {
    area: unsafe fn(*const ()) -> f64,
    name: fn() -> &'static str,
    drop: unsafe fn(*mut ()),
}
struct Obj {
    data: *mut (),
    vt: &'static VTable,
}
impl Obj {
    fn new<T: Shape>(v: T) -> Obj {
        unsafe fn area<T: Shape>(p: *const ()) -> f64 {
            (*(p as *const T)).area()
        }
        unsafe fn drop_it<T>(p: *mut ()) {
            drop(Box::from_raw(p as *mut T));
        }
        // promoted to 'static
        let vt: &'static VTable = &VTable { area: area::<T>, name: T::name, drop: drop_it::<T> };
        Obj { data: Box::into_raw(Box::new(v)) as *mut (), vt }
    }
}
impl Drop for Obj {
    fn drop(&mut self) {
        unsafe { (self.vt.drop)(self.data) }
    }
}
trait Shape {
    fn area(&self) -> f64;
    fn name() -> &'static str;
}
struct Sq(f64);
struct Tri(f64, f64);
impl Shape for Sq {
    fn area(&self) -> f64 { self.0 * self.0 }
    fn name() -> &'static str { "square" }
}
impl Shape for Tri {
    fn area(&self) -> f64 { 0.5 * self.0 * self.1 }
    fn name() -> &'static str { "triangle" }
}
impl Drop for Sq {
    fn drop(&mut self) { println!("drop Sq {}", self.0) }
}

fn main() {
    let arena = Arena::new(3);
    let mut refs = Vec::new();
    for i in 0..7 {
        let r = arena.alloc(Named { name: format!("n{}", i), next: ptr::null_mut() });
        refs.push(r as *mut Named);
    }
    println!("chunks {}", arena.chunks.borrow().len());
    let mut st = Stack { head: ptr::null_mut(), _m: PhantomData };
    for &r in refs.iter().step_by(2) {
        st.push(unsafe { &mut *r });
    }
    while let Some(n) = st.pop() {
        n.name.push('!');
        print!("{} ", n.name);
    }
    println!();

    let a = Sso::new("short");
    let b = Sso::new("this one is definitely longer than fifteen");
    println!("{} {} | {} {}", a.as_str(), a.is_inline(), b.as_str(), b.is_inline());
    println!("sso size {}", mem::size_of::<Sso>());
    let objs = vec![Obj::new(Sq(3.0)), Obj::new(Tri(4.0, 5.0)), Obj::new(Sq(1.5))];
    for o in &objs {
        println!("{} {}", (o.vt.name)(), unsafe { (o.vt.area)(o.data) });
    }
    drop(objs);
    let mut mu: [MaybeUninit<String>; 3] = [const { MaybeUninit::uninit() }; 3];
    mu[0].write("x".into());
    mu[1].write("y".into());
    let joined = unsafe { format!("{}{}", mu[0].assume_init_ref(), mu[1].assume_init_ref()) };
    unsafe {
        mu[0].assume_init_drop();
        mu[1].assume_init_drop();
    }
    println!("{}", joined);
    println!("arena dropping");
}
