//~ ERROR E0133
static mut COUNTER: u32 = 0;

fn main() {
    COUNTER += 1;
}
