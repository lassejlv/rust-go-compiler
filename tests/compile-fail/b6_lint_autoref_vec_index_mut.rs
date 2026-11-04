//~ ERROR lint
// dangerous_implicit_autorefs (deny by default): indexing a Vec through a
// raw pointer dereference implicitly takes `&mut *ptr`.
fn bump(raw_vec_ptr: *mut Vec<u32>, i: usize) {
    let slot = unsafe { &mut (*raw_vec_ptr)[i] };
    *slot += 1;
}

fn main() {
    let mut v = vec![1, 2, 3];
    bump(&mut v, 1);
    println!("{:?}", v);
}
