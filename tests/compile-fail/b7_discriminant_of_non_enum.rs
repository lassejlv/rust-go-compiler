//~ ERROR the return value of `mem::discriminant` is unspecified
// enum_intrinsics_non_enums is deny-by-default.
fn main() {
    let d = std::mem::discriminant(&5u32);
    println!("{:?}", d);
}
