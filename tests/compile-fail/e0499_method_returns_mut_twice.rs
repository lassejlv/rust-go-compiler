//~ ERROR E0499
struct Bag {
    items: Vec<i32>,
}

impl Bag {
    fn first_mut(&mut self) -> &mut i32 {
        &mut self.items[0]
    }
}

fn main() {
    let mut b = Bag { items: vec![1, 2] };
    let x = b.first_mut();
    let y = b.first_mut();
    *x += 1;
    *y += 1;
}
