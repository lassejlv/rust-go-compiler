// Lifetime elision: lifetimes inside `Self` (a by-value `self`, `Box<Self>`)
// are not input lifetime positions, so an elided lifetime in the output,
// also one inside a type alias (`Res<'_>`) or hidden in it (`-> Res`), is
// the lifetime of the only reference parameter (cexpr:
// `impl<'a> PRef<'a> { fn unary(self, input: &'_ [Token]) -> CResult<'_, EvalResult> }`).
type Res<'x> = Result<&'x [u8], ()>;
type Res2<'x, T> = Result<(&'x [u8], T), ()>;
#[derive(Debug)]
struct Wr<'a>(&'a [u8]);
#[derive(Clone, Copy)]
struct P<'a>(&'a str);
impl<'a> P<'a> {
    fn by_val(self, input: &'_ [u8]) -> Res<'_> {
        Ok(input)
    }
    fn hidden(self, input: &[u8]) -> Res {
        Ok(&input[..self.0.len().min(input.len())])
    }
    fn wrapped(self, input: &[u8]) -> Wr {
        Wr(input)
    }
    fn pair(self, input: &[u8]) -> Res2<'_, usize> {
        Ok((input, self.0.len()))
    }
    fn boxed(self: Box<Self>, input: &[u8]) -> Res2<'_, u8> {
        Ok((input, 1))
    }
    fn named(self, _input: &[u8]) -> &'a str {
        self.0
    }
}
struct Q;
impl Q {
    fn by_val(self, input: &'_ [u8]) -> Res<'_> {
        Ok(input)
    }
}
fn free(input: &[u8]) -> Res {
    Ok(input)
}
fn main() {
    let v = vec![1u8, 2, 3];
    let p = P("ab");
    println!("{:?} {:?}", p.by_val(&v), Q.by_val(&v));
    println!("{:?} {:?}", p.hidden(&v), p.wrapped(&v));
    println!("{:?} {:?}", p.pair(&v), Box::new(p).boxed(&v));
    println!("{} {:?}", p.named(&v), free(&v));
}
