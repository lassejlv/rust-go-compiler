//~ ERROR E0603
mod outer {
    mod inner { pub fn f() {} }
}
pub use outer::inner;
fn main() {}
