//~ ERROR E0594
static mut_counter: u32 = 0;
fn main() {
    mut_counter = 1;
}
