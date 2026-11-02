// Recursive tt-muncher macros: a tiny DSL parser, counting, reversing, internal rules (@acc),
// and building nested data structures at compile time.

// Count token trees recursively
macro_rules! count_tts {
    () => { 0usize };
    ($head:tt $($tail:tt)*) => { 1usize + count_tts!($($tail)*) };
}

// Reverse a list of expressions using an accumulator
macro_rules! reverse_vec {
    (@acc [$($acc:expr),*]) => { vec![$($acc),*] };
    (@acc [$($acc:expr),*] $head:expr $(, $tail:expr)*) => {
        reverse_vec!(@acc [$head $(, $acc)*] $($tail),*)
    };
    ($($e:expr),* $(,)?) => { reverse_vec!(@acc [] $($e),*) };
}

// A key-value DSL with optional types and defaults:  name: Type = default;
macro_rules! config_struct {
    (@field $name:ident : $ty:ty = $default:expr) => { $default };
    (@field $name:ident : $ty:ty) => { <$ty as Default>::default() };
    (
        struct $sname:ident {
            $( $fname:ident : $fty:ty $(= $fdefault:expr)? ; )*
        }
    ) => {
        #[derive(Debug, Clone)]
        struct $sname { $( $fname: $fty, )* }
        impl Default for $sname {
            fn default() -> Self {
                $sname { $( $fname: config_struct!(@field $fname : $fty $(= $fdefault)?), )* }
            }
        }
        impl $sname {
            fn field_names() -> &'static [&'static str] { &[$(stringify!($fname)),*] }
        }
    };
}

config_struct! {
    struct ServerConfig {
        host: String = "localhost".to_string();
        port: u16 = 8080;
        workers: usize;
        verbose: bool;
        ratio: f64 = 0.75;
    }
}

// An arithmetic expression muncher that evaluates  `1 plus 2 times 3` left to right
macro_rules! calc {
    (@go $acc:expr ;) => { $acc };
    (@go $acc:expr ; plus $n:literal $($rest:tt)*) => { calc!(@go ($acc + $n) ; $($rest)*) };
    (@go $acc:expr ; minus $n:literal $($rest:tt)*) => { calc!(@go ($acc - $n) ; $($rest)*) };
    (@go $acc:expr ; times $n:literal $($rest:tt)*) => { calc!(@go ($acc * $n) ; $($rest)*) };
    (@go $acc:expr ; squared $($rest:tt)*) => { calc!(@go ($acc * $acc) ; $($rest)*) };
    ($n:literal $($rest:tt)*) => { calc!(@go $n ; $($rest)*) };
}

// HTML-ish tree builder munching nested braces
#[derive(Debug)]
enum Html {
    Elem(&'static str, Vec<Html>),
    Text(String),
}

macro_rules! html {
    (@children [$($out:expr),*]) => { vec![$($out),*] };
    (@children [$($out:expr),*] $tag:ident { $($inner:tt)* } $($rest:tt)*) => {
        html!(@children [$($out,)* Html::Elem(stringify!($tag), html!(@children [] $($inner)*))] $($rest)*)
    };
    (@children [$($out:expr),*] $text:literal $($rest:tt)*) => {
        html!(@children [$($out,)* Html::Text($text.to_string())] $($rest)*)
    };
    (@children [$($out:expr),*] ($e:expr) $($rest:tt)*) => {
        html!(@children [$($out,)* Html::Text(format!("{}", $e))] $($rest)*)
    };
    ($tag:ident { $($inner:tt)* }) => { Html::Elem(stringify!($tag), html!(@children [] $($inner)*)) };
}

fn render(h: &Html) -> String {
    match h {
        Html::Text(t) => t.clone(),
        Html::Elem(tag, kids) => {
            let inner: Vec<String> = kids.iter().map(render).collect();
            format!("<{0}>{1}</{0}>", tag, inner.join(""))
        }
    }
}

// Generate a match-based lookup function from `key => value` pairs, munching one pair at a time
macro_rules! lookup_fn {
    (@arms $name:ident [$($arms:tt)*]) => {
        fn $name(k: &str) -> Option<u32> {
            match k { $($arms)* _ => None }
        }
    };
    (@arms $name:ident [$($arms:tt)*] $k:literal => $v:expr $(, $($rest:tt)*)?) => {
        lookup_fn!(@arms $name [$($arms)* $k => Some($v),] $($($rest)*)?);
    };
    ($name:ident { $($body:tt)* }) => { lookup_fn!(@arms $name [] $($body)*); };
}

lookup_fn!(http_code { "ok" => 200, "created" => 201, "not_found" => 404, "teapot" => 400 + 18 });

fn main() {
    println!("{}", count_tts!());
    println!("{}", count_tts!(a b c));
    println!("{}", count_tts!(fn main() { let x = 1; }));
    const N: usize = count_tts!(1 2 3 4 5 6 7);
    let arr = [0u8; N];
    println!("arr len {}", arr.len());

    println!("{:?}", reverse_vec![1, 2, 3, 4, 5]);
    println!("{:?}", reverse_vec!["a", "b"]);
    let empty: Vec<i32> = reverse_vec![];
    println!("{:?}", empty);

    let c = ServerConfig::default();
    println!("{:?}", c);
    println!("{:?}", ServerConfig::field_names());

    println!("{}", calc!(1 plus 2 times 3));
    println!("{}", calc!(10 minus 4 squared plus 1));
    println!("{}", calc!(7));

    let name = "world";
    let page = html!(html {
        head { title { "Test page" } }
        body {
            h1 { "Hello, " (name) "!" }
            ul { li { "one" } li { (1 + 1) } li { "three" } }
            p { }
        }
    });
    println!("{}", render(&page));
    if let Html::Elem(_, kids) = &page {
        println!("top-level children: {}", kids.len());
    }

    for k in ["ok", "teapot", "nope", "not_found"] {
        println!("{} -> {:?}", k, http_code(k));
    }
}
