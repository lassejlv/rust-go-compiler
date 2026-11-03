// tt-muncher that parses a tiny expression language into RPN-ish token strings
macro_rules! rpn {
    (@acc [$($out:tt)*]) => { vec![$($out)*] };
    (@acc [$($out:tt)*] $n:literal $($rest:tt)*) => { rpn!(@acc [$($out)* stringify!($n),] $($rest)*) };
    (@acc [$($out:tt)*] + $($rest:tt)*) => { rpn!(@acc [$($out)* "+",] $($rest)*) };
    (@acc [$($out:tt)*] * $($rest:tt)*) => { rpn!(@acc [$($out)* "*",] $($rest)*) };
    (@acc [$($out:tt)*] ( $($inner:tt)* ) $($rest:tt)*) => { rpn!(@acc [$($out)* "(",] $($inner)* @close $($rest)*) };
    (@acc [$($out:tt)*] @close $($rest:tt)*) => { rpn!(@acc [$($out)* ")",] $($rest)*) };
    (@acc [$($out:tt)*] $id:ident $($rest:tt)*) => { rpn!(@acc [$($out)* concat!("id:", stringify!($id)),] $($rest)*) };
    ($($t:tt)*) => { rpn!(@acc [] $($t)*) };
}

// key-value muncher building a struct-like debug string
macro_rules! kv {
    (@munch $s:ident ;) => {};
    (@munch $s:ident ; $k:ident = $v:expr , $($rest:tt)*) => {
        $s.push(format!("{}={:?}", stringify!($k), $v));
        kv!(@munch $s ; $($rest)*);
    };
    (@munch $s:ident ; $k:ident = $v:expr) => {
        $s.push(format!("{}={:?}", stringify!($k), $v));
    };
    ($($t:tt)*) => {{
        let mut s: Vec<String> = Vec::new();
        kv!(@munch s ; $($t)*);
        s.join(" ")
    }};
}

// reverse a list of token trees
macro_rules! reverse {
    ([] $($rev:tt)*) => { [$($rev),*] };
    ([$first:tt $($rest:tt)*] $($rev:tt)*) => { reverse!([$($rest)*] $first $($rev)*) };
}

// munch operators to compute a value left-to-right
macro_rules! calc {
    ($acc:expr ;) => { $acc };
    ($acc:expr ; plus $n:literal $($rest:tt)*) => { calc!(($acc + $n) ; $($rest)*) };
    ($acc:expr ; minus $n:literal $($rest:tt)*) => { calc!(($acc - $n) ; $($rest)*) };
    ($acc:expr ; times $n:literal $($rest:tt)*) => { calc!(($acc * $n) ; $($rest)*) };
    (start $n:literal $($rest:tt)*) => { calc!($n ; $($rest)*) };
}

// munch a mini "struct" description into a list of (name, type-name) pairs
macro_rules! fields {
    (@out [$($o:expr),*]) => { vec![$($o),*] };
    (@out [$($o:expr),*] $name:ident : $t:ty ; $($rest:tt)*) => {
        fields!(@out [$($o,)* (stringify!($name), std::any::type_name::<$t>().rsplit("::").next().unwrap())] $($rest)*)
    };
    ($($t:tt)*) => { fields!(@out [] $($t)*) };
}

fn main() {
    let v: Vec<&str> = rpn!(1 + x * (2 + y));
    println!("{:?}", v);
    println!("{}", kv!(a = 1, b = "two", c = vec![3, 4]));
    println!("{}", kv!(only = 'x'));
    println!("{:?}", reverse!([1 2 3 4 5]));
    println!("{}", calc!(start 10 plus 5 times 3 minus 7));
    let f: Vec<(&str, &str)> = fields!(id: u64; name: String; ok: bool;);
    println!("{:?}", f);
}
