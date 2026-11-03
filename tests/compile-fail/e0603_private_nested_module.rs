//~ ERROR E0603
mod outer {
    mod inner {
        pub fn f() {}
    }
}

fn main() {
    outer::inner::f();
}
