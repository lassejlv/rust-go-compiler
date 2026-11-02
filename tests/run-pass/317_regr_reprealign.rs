#[repr(align(64))] struct A(u8);
#[repr(C, align(16))] struct B { x: u32 }
fn main() { println!("{} {} {} {}", std::mem::size_of::<A>(), std::mem::align_of::<A>(), std::mem::align_of::<B>(), &A(1) as *const A as usize % 64); }
