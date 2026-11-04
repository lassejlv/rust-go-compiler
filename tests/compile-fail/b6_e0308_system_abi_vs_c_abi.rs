//~ ERROR E0308
// `extern "system"` and `extern "C"` fn pointers are distinct types.
extern "system" fn handler(code: u32) -> u32 { code }

fn main() {
    let h: extern "C" fn(u32) -> u32 = handler;
    println!("{}", h(3));
}
