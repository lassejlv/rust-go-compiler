//~ ERROR `$p:pat` is followed by `|`, which is not allowed for `pat` fragments
// Since Rust 2021 `pat` matches or-patterns, so it may not be followed by
// `|` (here as the separator of its repetition); `pat_param` may.
macro_rules! any_of {
    ($e:expr, $($p:pat)|+) => {
        match $e { $($p)|+ => true, _ => false }
    };
}

fn main() {
    println!("{}", any_of!(3, 1 | 3));
}
