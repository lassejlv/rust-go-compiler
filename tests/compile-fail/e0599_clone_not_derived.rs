//~ ERROR E0599
struct Data {
    v: Vec<i32>,
}

fn main() {
    let d = Data { v: vec![1] };
    let e = d.clone();
    println!("{}", e.v.len());
}
