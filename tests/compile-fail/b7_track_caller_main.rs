//~ ERROR `main` function is not allowed to be `#[track_caller]`
// `main` cannot be `#[track_caller]`.
#[track_caller]
fn main() {}
