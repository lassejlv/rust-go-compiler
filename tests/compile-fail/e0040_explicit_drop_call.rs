//~ ERROR E0040
struct R;

impl Drop for R {
    fn drop(&mut self) {}
}

fn main() {
    let mut r = R;
    r.drop();
}
