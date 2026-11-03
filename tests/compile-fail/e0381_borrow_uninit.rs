//~ ERROR E0381
fn show(v: &Vec<i32>) {
    println!("{:?}", v);
}

fn main() {
    let v: Vec<i32>;
    show(&v);
}
