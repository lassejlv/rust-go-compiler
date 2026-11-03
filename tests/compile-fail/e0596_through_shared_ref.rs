//~ ERROR E0596
fn add(v: &Vec<i32>) {
    v.push(1);
}

fn main() {
    let mut v = vec![];
    add(&v);
    println!("{:?}", v);
}
