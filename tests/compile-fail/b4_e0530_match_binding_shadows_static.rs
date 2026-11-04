//~ ERROR E0530
static LIMIT: i32 = 10;
fn main() {
    match 5 {
        LIMIT => println!("limit"),
        _ => println!("other"),
    }
}
