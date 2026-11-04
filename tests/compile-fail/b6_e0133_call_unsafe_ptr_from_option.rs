//~ ERROR E0133
// An optional unsafe callback still needs unsafe to be called.
type Hook = unsafe fn(&mut Vec<u8>);

unsafe fn clear_hook(v: &mut Vec<u8>) { v.clear() }

fn main() {
    let hook: Option<Hook> = Some(clear_hook);
    let mut v = vec![1, 2, 3];
    if let Some(h) = hook {
        h(&mut v);
    }
    println!("{:?}", v);
}
