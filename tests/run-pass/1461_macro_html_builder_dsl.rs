// a recursive HTML-builder DSL macro with nested element/attribute munching
use std::fmt::Write;

macro_rules! html {
    (@attrs $out:ident) => {};
    (@attrs $out:ident $name:ident = $val:expr $(, $($rest:tt)*)?) => {
        write!($out, " {}=\"{}\"", stringify!($name), $val).unwrap();
        html!(@attrs $out $($($rest)*)?);
    };
    (@nodes $out:ident) => {};
    (@nodes $out:ident $text:literal $($rest:tt)*) => {
        write!($out, "{}", $text).unwrap();
        html!(@nodes $out $($rest)*);
    };
    (@nodes $out:ident {$e:expr} $($rest:tt)*) => {
        write!($out, "{}", $e).unwrap();
        html!(@nodes $out $($rest)*);
    };
    (@nodes $out:ident for $x:ident in $it:expr => [ $($body:tt)* ] $($rest:tt)*) => {
        for $x in $it { html!(@nodes $out $($body)*); }
        html!(@nodes $out $($rest)*);
    };
    (@nodes $out:ident $tag:ident ( $($attrs:tt)* ) [ $($children:tt)* ] $($rest:tt)*) => {
        write!($out, "<{}", stringify!($tag)).unwrap();
        html!(@attrs $out $($attrs)*);
        write!($out, ">").unwrap();
        html!(@nodes $out $($children)*);
        write!($out, "</{}>", stringify!($tag)).unwrap();
        html!(@nodes $out $($rest)*);
    };
    (@nodes $out:ident $tag:ident [ $($children:tt)* ] $($rest:tt)*) => {
        html!(@nodes $out $tag () [ $($children)* ] $($rest)*);
    };
    (@nodes $out:ident $tag:ident / $($rest:tt)*) => {
        write!($out, "<{}/>", stringify!($tag)).unwrap();
        html!(@nodes $out $($rest)*);
    };
    ($($t:tt)*) => {{
        let mut s = String::new();
        {
            let out = &mut s;
            html!(@nodes out $($t)*);
        }
        s
    }};
}

fn main() {
    let title = "Hi & bye";
    let items = vec!["one", "two", "three"];
    let page = html! {
        html [
            head [ title [ {title} ] ]
            body(class = "main", id = 7) [
                h1 [ "Header" ]
                br /
                ul [
                    for it in items.iter() => [ li(data = it.len()) [ {it} ] ]
                ]
                p [ "n=" {items.len()} ]
            ]
        ]
    };
    println!("{}", page);
}
