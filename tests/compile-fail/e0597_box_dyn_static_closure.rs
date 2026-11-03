//~ ERROR E0597
fn make() -> Box<dyn Fn() -> usize> {
    Box::new(|| 0)
}

fn main() {
    let mut f = make();
    let v = vec![1, 2, 3];
    let g: Box<dyn Fn() -> usize + 'static> = Box::new(|| v.len());
    f = g;
    println!("{}", f());
}
