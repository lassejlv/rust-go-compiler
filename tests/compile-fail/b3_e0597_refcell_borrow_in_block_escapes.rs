//~ ERROR E0597
use std::cell::RefCell;
fn main() {
    let r;
    {
        let cell = RefCell::new(5);
        r = *cell.borrow() + 0;
        let g = cell.borrow();
        let _keep: &i32 = &g;
    }
    let out: &i32;
    {
        let x = 10;
        out = &x;
    }
    println!("{} {}", r, out);
}
