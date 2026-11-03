//~ ERROR lifetime may not live long enough
struct Slot<'r, 'a> {
    slot: &'r mut &'a str,
}

impl<'r, 'a> Slot<'r, 'a> {
    fn set<'b>(&mut self, x: &'b str) {
        *self.slot = x;
    }
}

fn main() {
    let mut s = "s";
    let mut sl = Slot { slot: &mut s };
    sl.set("t");
    println!("{}", s);
}
