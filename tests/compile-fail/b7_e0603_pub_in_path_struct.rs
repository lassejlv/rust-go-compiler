//~ ERROR E0603
// A struct restricted to an ancestor module used outside of it.
mod outer {
    pub mod inner {
        pub(in crate::outer) struct Token;
    }
    pub fn make() {
        let _t = inner::Token;
    }
}
fn main() {
    outer::make();
    let _t = outer::inner::Token;
}
