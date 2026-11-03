//~ ERROR E0597
struct Item<'a> { name: &'a str }
fn main() {
    let mut items: Vec<Item> = Vec::new();
    {
        let s = String::from("tmp");
        items.push(Item { name: &s });
    }
    println!("{}", items.len());
}
