// eager expansion of user macros inside concat!/include_str!/env!/stringify-like builtins
macro_rules! name { () => { "1476_macro_eager_builtin_args" }; }
macro_rules! ext { () => { ".rs" }; }
macro_rules! file_name { () => { concat!(name!(), ext!()) }; }
macro_rules! version { () => { 3 }; }
macro_rules! tag { ($t:ident) => { stringify!($t) }; }
macro_rules! nested_concat { ($($x:expr),*) => { concat!($($x, "/"),*) }; }

mod m {
    macro_rules! local_lit { () => { "local" }; }
    pub const S: &str = concat!("m:", local_lit!(), ":", crate::exported_lit!());
}

#[macro_export]
macro_rules! exported_lit { () => { "exp" }; }

fn main() {
    println!("{}", concat!("v", version!(), "-", tag!(beta)));
    println!("{}", file_name!());
    let src = include_str!(file_name!());
    println!("{}", src.lines().next().unwrap());
    let src2 = include_str!(concat!(name!(), ext!()));
    println!("{}", src2.len() == src.len());
    println!("{}", m::S);
    println!("{}", nested_concat!(name!(), 1, true, tag!(x)));
    println!("{}", concat!(line!(), ":", column!()));
    println!("{:?}", option_env!(concat!("NO_SUCH_", "VAR_", "AT_ALL")));
}
