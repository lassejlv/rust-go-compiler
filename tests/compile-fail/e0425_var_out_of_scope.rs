//~ ERROR E0425
fn main() {
    {
        let inner = 5;
        println!("{}", inner);
    }
    println!("{}", inner);
}
