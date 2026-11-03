//~ ERROR E0451
mod cfg {
    #[derive(Default)]
    pub struct Opts { pub verbose: bool, level: u8 }
}
fn main() {
    let o = cfg::Opts { verbose: true, level: 2 };
    let _ = o.verbose;
}
