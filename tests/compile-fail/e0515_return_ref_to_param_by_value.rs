//~ ERROR E0515
fn head(v: Vec<i32>) -> &'static i32 {
    &v[0]
}

fn main() {
    println!("{}", head(vec![1]));
}
