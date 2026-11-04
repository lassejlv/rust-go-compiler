// tt-muncher parsing a tiny DSL, internal rules, accumulator
macro_rules! calc {
    (@go [$($acc:tt)*]) => { $($acc)* };
    (@go [$($acc:tt)*] plus $n:literal $($rest:tt)*) => { calc!(@go [($($acc)* + $n)] $($rest)*) };
    (@go [$($acc:tt)*] times $n:literal $($rest:tt)*) => { calc!(@go [($($acc)* * $n)] $($rest)*) };
    (@go [$($acc:tt)*] minus ($($e:tt)*) $($rest:tt)*) => { calc!(@go [($($acc)* - calc!($($e)*))] $($rest)*) };
    ($n:literal $($rest:tt)*) => { calc!(@go [$n] $($rest)*) };
}

macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! count_fast {
    (@unit $x:tt) => { () };
    ($($x:tt)*) => { <[()]>::len(&[$(count_fast!(@unit $x)),*]) };
}

macro_rules! reverse {
    (@acc [$($r:expr),*]) => { [$($r),*] };
    (@acc [$($r:expr),*] $h:expr $(, $t:expr)*) => { reverse!(@acc [$h $(, $r)*] $($t),*) };
    ($($x:expr),* $(,)?) => { reverse!(@acc [] $($x),*) };
}

macro_rules! kv_struct {
    ($name:ident { $($field:ident : $ty:ty = $def:expr),* $(,)? }) => {
        #[derive(Debug)]
        struct $name { $($field: $ty),* }
        impl $name {
            fn new() -> Self { $name { $($field: $def),* } }
            fn field_names() -> &'static [&'static str] { &[$(stringify!($field)),*] }
        }
    };
}

kv_struct!(Config { width: u32 = 80, height: u32 = 24, title: String = String::from("term"), });

macro_rules! tuple_index {
    ($t:expr; $($i:tt),*) => { ($($t.$i,)*) };
}

fn main() {
    println!("{}", calc!(1 plus 2 times 3));
    println!("{}", calc!(10 minus (2 plus 3) times 2));
    println!("{}", count!(a b c d e f g));
    println!("{}", count_fast!(a , b ; c [d e] {f} g));
    let r = reverse!(1, 2, 3, 4, 5,);
    println!("{:?}", r);
    let c = Config::new();
    println!("{:?} {:?}", c, Config::field_names());
    let t = (1, "two", 3.5, 'x');
    println!("{:?}", tuple_index!(t; 3, 1, 0));
    const N: usize = count!(x y z);
    let arr = [0u8; N];
    println!("{}", arr.len());
}
