use std::mem::MaybeUninit;

#[derive(Debug)]
struct Noisy(u32);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop Noisy({})", self.0);
    }
}

fn make_array() -> [String; 4] {
    let mut arr: [MaybeUninit<String>; 4] = unsafe { MaybeUninit::uninit().assume_init() };
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write(format!("item{}", i * i));
    }
    unsafe { std::mem::transmute::<_, [String; 4]>(arr) }
}

fn main() {
    let a = make_array();
    println!("{:?}", a);

    let mut x = MaybeUninit::<u64>::uninit();
    unsafe { x.as_mut_ptr().write(0xdead_beef); }
    let x = unsafe { x.assume_init() };
    println!("{:x}", x);

    let arr2: [MaybeUninit<Noisy>; 3] = [const { MaybeUninit::uninit() }; 3];
    let mut arr2 = arr2;
    arr2[0].write(Noisy(1));
    arr2[1].write(Noisy(2));
    // only drop the initialized ones
    unsafe {
        arr2[0].assume_init_drop();
        let n = arr2[1].assume_init_read();
        println!("read {:?}", n);
    }
    println!("after");

    let z: MaybeUninit<[u8; 4]> = MaybeUninit::zeroed();
    println!("{:?}", unsafe { z.assume_init() });

    let mut buf = [MaybeUninit::<i32>::uninit(); 5];
    for i in 0..5 {
        buf[i] = MaybeUninit::new((i as i32 + 1) * 3);
    }
    let vals: Vec<i32> = buf.iter().map(|m| unsafe { m.assume_init() }).collect();
    println!("{:?}", vals);
    let init = MaybeUninit::new(Noisy(9));
    let inner = unsafe { init.assume_init() };
    println!("inner {}", inner.0);
}
