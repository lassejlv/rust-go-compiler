//~ ERROR E0432
mod outer {
    pub mod inner { use super::helper; pub fn f() { helper(); } }
}
fn main() { outer::inner::f(); }
