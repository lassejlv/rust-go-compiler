//~ ERROR E0594
// A variable moved into a closure is as mutable as its binding.
fn main() {
    let count = 0;
    let mut bump = move || {
        count = count + 1;
        count
    };
    println!("{}", bump());
}
