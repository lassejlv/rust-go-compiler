//~ ERROR E0507
struct Wrapper {
    inner: String,
}

fn take(w: &Wrapper) -> String {
    w.inner
}

fn main() {
    let w = Wrapper { inner: "i".into() };
    println!("{}", take(&w));
}
