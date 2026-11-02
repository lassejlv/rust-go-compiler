trait Size { type ArrayType<T>; }
struct U3;
impl Size for U3 { type ArrayType<T> = [T; 3]; }
fn first<T: Copy, U, const N: usize>(a: U::ArrayType<T>) -> T where U: Size<ArrayType<T> = [T; N]> { a[0] }
fn main() { println!("{}", first::<i32, U3, 3>([7, 8, 9])); }
