macro_rules! count {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count!($($tail)*) };
}

macro_rules! count_fast {
    () => { 0usize };
    ($odd:tt $($a:tt $b:tt)*) => { (count_fast!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_fast!($($a)*) << 1 };
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => { $sub };
}
macro_rules! count_slice {
    ($($t:tt)*) => { <[()]>::len(&[$(replace_expr!($t ())),*]) };
}

macro_rules! enum_with_count {
    ($name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        enum $name { $($variant),* }
        impl $name {
            const COUNT: usize = count!($($variant)*);
            const ALL: [$name; count!($($variant)*)] = [$($name::$variant),*];
            fn index(self) -> usize { self as usize }
            fn name(self) -> &'static str {
                match self { $($name::$variant => stringify!($variant)),* }
            }
        }
    };
}

enum_with_count!(Planet { Mercury, Venus, Earth, Mars, Jupiter, });

macro_rules! tuple_len {
    ($($e:expr),*) => { count!($($e)*) };
}

macro_rules! indexed {
    (@step $i:expr ; ) => {};
    (@step $i:expr ; $head:ident $($tail:ident)*) => {
        println!("{} = {}", stringify!($head), $i);
        indexed!(@step $i + 1 ; $($tail)*);
    };
    ($($id:ident)*) => { indexed!(@step 0 ; $($id)*); };
}

fn main() {
    println!("{}", count!(a b c d e f g));
    println!("{}", count_fast!(1 2 3 4 5 6 7 8 9 10 11 12 13));
    println!("{}", count_fast!());
    println!("{}", count_slice!(x y z w));
    println!("{} {:?}", Planet::COUNT, Planet::ALL);
    for p in Planet::ALL.iter().rev() {
        print!("{}:{} ", p.name(), p.index());
    }
    println!();
    println!("{}", tuple_len!(1 + 2, "s", [1, 2], (3, 4)));
    indexed!(zero one two three);
    const N: usize = count!(1 2 3);
    let arr = [0u8; N];
    println!("arr len {}", arr.len());
}
