// A hand-rolled Arc/Weak (like triomphe or std's Arc) using NonNull, AtomicUsize refcounts,
// fences, Box::into_raw/from_raw, Deref, get_mut/make_mut, and Send/Sync impls, used across threads.
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{fence, AtomicUsize, Ordering};
use std::thread;

struct Inner<T> {
    strong: AtomicUsize,
    weak: AtomicUsize, // +1 held collectively by all strong refs
    data: std::mem::ManuallyDrop<T>,
}

struct MyArc<T> {
    ptr: NonNull<Inner<T>>,
    _p: PhantomData<Inner<T>>,
}

struct MyWeak<T> {
    ptr: NonNull<Inner<T>>,
}

unsafe impl<T: Send + Sync> Send for MyArc<T> {}
unsafe impl<T: Send + Sync> Sync for MyArc<T> {}
unsafe impl<T: Send + Sync> Send for MyWeak<T> {}
unsafe impl<T: Send + Sync> Sync for MyWeak<T> {}

impl<T> MyArc<T> {
    fn new(v: T) -> Self {
        let b = Box::new(Inner { strong: AtomicUsize::new(1), weak: AtomicUsize::new(1), data: std::mem::ManuallyDrop::new(v) });
        MyArc { ptr: NonNull::from(Box::leak(b)), _p: PhantomData }
    }
    fn inner(&self) -> &Inner<T> {
        unsafe { self.ptr.as_ref() }
    }
    fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(Ordering::SeqCst)
    }
    fn weak_count(this: &Self) -> usize {
        this.inner().weak.load(Ordering::SeqCst) - 1
    }
    fn downgrade(this: &Self) -> MyWeak<T> {
        this.inner().weak.fetch_add(1, Ordering::Relaxed);
        MyWeak { ptr: this.ptr }
    }
    fn get_mut(this: &mut Self) -> Option<&mut T> {
        let i = this.inner();
        if i.strong.load(Ordering::Acquire) == 1 && i.weak.load(Ordering::Acquire) == 1 {
            unsafe { Some(&mut *(&mut (*this.ptr.as_ptr()).data as *mut std::mem::ManuallyDrop<T> as *mut T)) }
        } else {
            None
        }
    }
    fn make_mut(this: &mut Self) -> &mut T
    where
        T: Clone,
    {
        if MyArc::get_mut(this).is_none() {
            *this = MyArc::new((**this).clone());
        }
        MyArc::get_mut(this).unwrap()
    }
    fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.ptr == b.ptr
    }
}

impl<T> MyWeak<T> {
    fn upgrade(&self) -> Option<MyArc<T>> {
        let inner = unsafe { self.ptr.as_ref() };
        let mut n = inner.strong.load(Ordering::Relaxed);
        loop {
            if n == 0 {
                return None;
            }
            match inner.strong.compare_exchange_weak(n, n + 1, Ordering::Acquire, Ordering::Relaxed) {
                Ok(_) => return Some(MyArc { ptr: self.ptr, _p: PhantomData }),
                Err(cur) => n = cur,
            }
        }
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        self.inner().strong.fetch_add(1, Ordering::Relaxed);
        MyArc { ptr: self.ptr, _p: PhantomData }
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner().data
    }
}

impl<T> Drop for MyArc<T> {
    fn drop(&mut self) {
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        fence(Ordering::Acquire);
        unsafe {
            std::mem::ManuallyDrop::drop(&mut (*self.ptr.as_ptr()).data);
        }
        // drop the implicit weak held by strong refs
        drop(MyWeak { ptr: self.ptr });
    }
}

impl<T> Drop for MyWeak<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if inner.weak.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

struct Tracked(&'static str, Vec<u32>);
impl Drop for Tracked {
    fn drop(&mut self) {
        println!("drop Tracked({}) with {} items", self.0, self.1.len());
    }
}
impl Clone for Tracked {
    fn clone(&self) -> Self {
        println!("clone Tracked({})", self.0);
        Tracked(self.0, self.1.clone())
    }
}

fn main() {
    let a = MyArc::new(Tracked("shared", (1..=100).collect()));
    let w = MyArc::downgrade(&a);
    println!("strong={} weak={}", MyArc::strong_count(&a), MyArc::weak_count(&a));
    let sums: Vec<u32> = thread::scope(|s| {
        (0..4)
            .map(|t| {
                let a = a.clone();
                s.spawn(move || a.1.iter().filter(|x| *x % 4 == t).sum::<u32>())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });
    println!("sums {:?} total {}", sums, sums.iter().sum::<u32>());
    println!("after threads strong={}", MyArc::strong_count(&a));

    let handles: Vec<_> = (0..3)
        .map(|i| {
            let w2 = MyArc::downgrade(&a);
            thread::spawn(move || w2.upgrade().map(|s| s.1[i * 10]).unwrap_or(0))
        })
        .collect();
    let picked: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    println!("picked {:?} weak now {}", picked, MyArc::weak_count(&a));

    let mut b = a.clone();
    println!("ptr_eq {}", MyArc::ptr_eq(&a, &b));
    MyArc::make_mut(&mut b).1.push(101);
    println!("after make_mut: ptr_eq {} a.len {} b.len {}", MyArc::ptr_eq(&a, &b), a.1.len(), b.1.len());
    println!("get_mut on b (unique) {}", MyArc::get_mut(&mut b).is_some());
    let mut a = a;
    println!("get_mut on a (has weak) {}", MyArc::get_mut(&mut a).is_some());
    drop(b);
    println!("upgrade before drop: {}", w.upgrade().map(|x| x.0).unwrap_or("none"));
    drop(a);
    println!("upgrade after drop: {}", w.upgrade().map(|x| x.0).unwrap_or("none"));
    drop(w);

    let counter = MyArc::new(AtomicUsize::new(0));
    thread::scope(|s| {
        for _ in 0..8 {
            let c = counter.clone();
            s.spawn(move || {
                for _ in 0..1000 {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    println!("counter {} strong {}", counter.load(Ordering::SeqCst), MyArc::strong_count(&counter));
    println!("end");
}
