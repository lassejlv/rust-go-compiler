#![recursion_limit = "300"]
// deep recursion needs a raised recursion_limit
macro_rules! count_tts {
    () => { 0usize };
    ($odd:tt $($a:tt $b:tt)*) => { (count_tts!($($a)*) << 1) | 1 };
    ($($a:tt $even:tt)*) => { count_tts!($($a)*) << 1 };
}

macro_rules! slow_count {
    () => { 0 };
    ($h:tt $($t:tt)*) => { 1 + slow_count!($($t)*) };
}

macro_rules! build_list {
    (@acc [$($acc:expr),*]) => { vec![$($acc),*] };
    (@acc [$($acc:expr),*] $h:literal $($t:literal)*) => { build_list!(@acc [$($acc,)* $h * 2] $($t)*) };
}

macro_rules! many {
    ($($t:tt)*) => { slow_count!($($t)* $($t)* $($t)*) };
}

fn main() {
    println!("{}", count_tts!(a b c d e f g h i j k l m n o p q r s t u v w x y z));
    println!("{}", slow_count!(
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9
    ));
    let l = build_list!(@acc [] 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20);
    println!("{:?}", l);
    println!("{}", many!(a b c d e f g h i j k l m n o p q r s t u v w x y z 1 2 3 4 5 6 7 8 9 0 a b c d e f g h i j k l m n o p q r s t u v w x y z));
}
