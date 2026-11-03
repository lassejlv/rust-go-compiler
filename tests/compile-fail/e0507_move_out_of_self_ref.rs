//~ ERROR E0507
struct Holder {
    items: Vec<i32>,
}

impl Holder {
    fn into_items(&self) -> Vec<i32> {
        self.items
    }
}

fn main() {
    let h = Holder { items: vec![1] };
    println!("{:?}", h.into_items());
}
