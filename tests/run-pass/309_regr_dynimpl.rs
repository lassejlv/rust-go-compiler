use std::fmt;
trait Expected { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result; }
impl Expected for &str { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str(self) } }
impl fmt::Display for dyn Expected + '_ {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { Expected::fmt(self, f) }
}
fn main() { let e: &dyn Expected = &"hi"; println!("{}", e); }
