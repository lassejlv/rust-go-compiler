//~ ERROR E0507
fn first(v: &Vec<String>) -> String {
    let r = &v[0];
    *r
}

fn main() {
    println!("{}", first(&vec![String::from("z")]));
}
