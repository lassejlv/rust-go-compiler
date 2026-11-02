enum Imp<'a> { Owned(Box<[u8]>), Borrowed(&'a [u8]) }
impl<'a> Imp<'a> {
    fn as_slice(&self) -> &[u8] {
        #[cfg(unix)]
        {
            match self {
                Imp::Owned(ref x) => x,
                Imp::Borrowed(x) => x,
            }
        }
        #[cfg(not(unix))]
        {
            &[]
        }
    }
}
fn main() {
    let a = Imp::Owned(vec![1u8, 2].into_boxed_slice());
    let b = Imp::Borrowed(b"xyz");
    println!("{:?} {:?}", a.as_slice(), b.as_slice());
}
