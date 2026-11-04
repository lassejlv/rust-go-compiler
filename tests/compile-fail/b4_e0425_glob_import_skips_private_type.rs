//~ ERROR E0425
// A glob import only brings the names visible from the importing module.
mod shapes {
    pub struct Circle;
    struct Secret;
}
use shapes::*;
fn main() {
    let _c = Circle;
    let _s: Secret;
}
