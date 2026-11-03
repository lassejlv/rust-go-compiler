macro_rules! make_adder_macro {
    ($name:ident, $n:expr) => {
        macro_rules! $name {
            ($x:expr) => { $x + $n };
        }
    };
}
make_adder_macro!(add10, 10);
make_adder_macro!(add_neg, -3);

// pass `$` in as a token so the inner macro can have its own repetitions
macro_rules! make_lister {
    ($d:tt $name:ident, $sep:literal) => {
        macro_rules! $name {
            ($d($d item:expr),*) => {{
                let parts: Vec<String> = vec![$d($d item.to_string()),*];
                parts.join($sep)
            }};
        }
    };
}

macro_rules! gen_getters {
    ($struct:ident { $($f:ident : $t:ty),* }) => {
        struct $struct { $($f: $t),* }
        impl $struct {
            $(
                fn $f(&self) -> &$t { &self.$f }
            )*
        }
        macro_rules! describe_fields {
            () => { concat!($(stringify!($f), ":", stringify!($t), ";"),*) };
        }
    };
}
gen_getters!(Point { x: i32, y: i32, label: String });

mod inner {
    #[macro_export]
    macro_rules! exported_helper {
        () => { $crate::inner::helper_value() };
    }
    pub fn helper_value() -> u32 { 4242 }
}

macro_rules! use_crate_path {
    ($v:expr) => { $crate::double($v) };
}
fn double(x: i32) -> i32 { x * 2 }

macro_rules! shadow_check {
    ($e:expr) => {{
        let x = 100;
        $e + x
    }};
}

fn main() {
    println!("{}", add10!(5));
    println!("{}", add_neg!(add10!(1)));
    make_lister!($ comma_list, ", ");
    make_lister!($ pipe_list, "|");
    println!("{}", comma_list!(1, "two", 3.5));
    println!("{}", pipe_list!('a', true));
    println!("[{}]", pipe_list!());
    let p = Point { x: 3, y: -4, label: "P".into() };
    println!("{} {} {}", p.x(), p.y(), p.label());
    println!("{}", describe_fields!());
    println!("{}", exported_helper!());
    println!("{}", crate::exported_helper!());
    println!("{}", use_crate_path!(21));
    let x = 1;
    println!("hygiene {}", shadow_check!(x * 2));
}
