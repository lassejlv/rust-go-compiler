//~ ERROR E0603
mod m {
    pub struct Id(u32);
}

fn main() {
    let _i = m::Id(3);
}
