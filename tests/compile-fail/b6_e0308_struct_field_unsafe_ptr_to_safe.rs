//~ ERROR E0308
// A struct field of safe fn pointer type initialized from an unsafe one.
struct Hooks {
    on_drop: fn(usize),
}

unsafe fn release(_addr: usize) {}

fn main() {
    let h = Hooks { on_drop: release };
    (h.on_drop)(0);
}
