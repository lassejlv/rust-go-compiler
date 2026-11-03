//~ ERROR E0381
fn main() {
    let last: String;
    for i in 0..3 {
        if i == 10 {
            last = i.to_string();
            break;
        }
    }
    println!("{}", last);
}
