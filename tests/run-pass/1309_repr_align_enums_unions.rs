use std::mem::{align_of, size_of};
#[repr(align(8))]
#[derive(Debug, Clone, Copy, PartialEq)]
enum C { A, B, Cc }
#[repr(align(16))]
#[derive(Debug)]
enum N { Some(Box<u8>), None }
#[repr(u8, align(4))]
#[derive(Debug)]
enum T { X(u8), Y }
#[repr(C, align(16))]
union U { a: u8, b: u32 }
#[repr(align(2))]
enum One { Only(u8) }
fn main() {
    println!("{} {}", size_of::<C>(), align_of::<C>());
    println!("{} {}", size_of::<Option<C>>(), align_of::<Option<C>>());
    println!("{} {}", size_of::<N>(), align_of::<N>());
    println!("{} {}", size_of::<Option<N>>(), align_of::<Option<N>>());
    println!("{} {}", size_of::<T>(), align_of::<T>());
    println!("{} {}", size_of::<U>(), align_of::<U>());
    println!("{} {}", size_of::<One>(), align_of::<One>());
    println!("{} {}", size_of::<(u8, C)>(), size_of::<[C; 3]>());
    let v = vec![C::A, C::B, C::Cc, C::B];
    println!("{:?} {}", v, v.as_ptr() as usize % 8);
    let n = [N::Some(Box::new(4)), N::None];
    println!("{:?} {}", n, &n[1] as *const N as usize % 16);
    let t = [T::X(3), T::Y];
    println!("{:?}", t);
    let u = U { b: 0x01020304 };
    println!("{}", unsafe { u.a });
    let o = One::Only(9);
    let One::Only(x) = o;
    println!("{}", x);
    let mut oc = Some(C::B);
    if let Some(c) = oc.as_mut() { *c = C::Cc; }
    println!("{:?} {}", oc, oc == Some(C::Cc));
}
