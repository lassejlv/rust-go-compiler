//~ ERROR E0364
mod m {
    fn hidden() {}
    pub use self::hidden as visible;
}
fn main() {}
