// ControlFlow::map_break/map_continue; {Box,Rc,Arc}::<[T]>::new_uninit_slice /
// new_zeroed_slice live on the slice types like std (so `Box::<[u8]>::...` works).
use std::mem::MaybeUninit;
use std::ops::ControlFlow;
use std::rc::Rc;
use std::sync::Arc;

fn first_negative(v: &[i32]) -> ControlFlow<i32> {
    v.iter().try_for_each(|x| if *x < 0 { ControlFlow::Break(*x) } else { ControlFlow::Continue(()) })
}

fn main() {
    let c = first_negative(&[3, -4, 5]).map_break(|x| x * 10);
    println!("{:?} {:?}", c, c.break_value());
    let d: ControlFlow<i32, u8> = ControlFlow::Continue(2);
    println!("{:?}", d.map_continue(|x| x as u32 + 1).map_break(|b| b.to_string()));
    println!("{:?}", first_negative(&[1]).map_break(|x| x + 1).is_continue());

    let mut b = Box::<[u8]>::new_uninit_slice(3);
    for (i, x) in b.iter_mut().enumerate() {
        x.write(i as u8 * 2);
    }
    println!("{:?}", unsafe { b.assume_init() });
    let z: Box<[MaybeUninit<u32>]> = Box::new_zeroed_slice(2);
    println!("{:?}", unsafe { z.assume_init() });
    let mut r = Rc::<[u16]>::new_uninit_slice(2);
    for x in Rc::get_mut(&mut r).unwrap() {
        x.write(9);
    }
    println!("{:?}", unsafe { r.assume_init() });
    let a = Arc::<[i64]>::new_zeroed_slice(3);
    println!("{:?}", unsafe { a.assume_init() });
    let mut a2: Arc<[MaybeUninit<char>]> = Arc::new_uninit_slice(1);
    Arc::get_mut(&mut a2).unwrap()[0].write('x');
    println!("{:?}", unsafe { a2.assume_init() });
}
