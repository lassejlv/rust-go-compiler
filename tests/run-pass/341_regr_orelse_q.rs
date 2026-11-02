use std::io;
#[derive(Debug)] struct Errno(i32);
impl From<Errno> for io::Error { fn from(e: Errno) -> io::Error { io::Error::from_raw_os_error(e.0) } }
fn pipe_with(fail: bool) -> Result<(i32, i32), Errno> { if fail { Err(Errno(22)) } else { Ok((3, 4)) } }
fn pipe() -> Result<(i32, i32), Errno> { Ok((5, 6)) }
fn getfd(x: i32) -> Result<i32, Errno> { Ok(x * 10) }
fn make(fail: bool) -> io::Result<(i32, i32)> {
    let (r, w) = pipe_with(fail).or_else(|_| {
        let (r, w) = pipe()?;
        let _ = getfd(r)? | 1;
        io::Result::Ok((r, w))
    })?;
    Ok((r, w))
}
fn main() { println!("{:?} {:?}", make(false).unwrap(), make(true).unwrap()); }
