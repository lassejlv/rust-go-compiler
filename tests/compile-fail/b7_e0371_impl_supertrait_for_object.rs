//~ ERROR E0371
// `dyn Circle` implements its supertrait `Shape` too.
trait Shape {}
trait Circle: Shape {}
impl Shape for dyn Circle {}
fn main() {}
