// edition:2018
// Edition 2018 (vs 2021): single-argument panic!/assert!/unreachable!
// messages are not format strings (and panic!(expr) panics with expr as the
// payload), `array.into_iter()` and `boxed_slice.into_iter()` iterate by
// reference while `Box<[T; N]>` iterates by value, `c"x"` is an identifier
// followed by a string, `$p:pat` stops at `|`, and `try`/`async` macros and
// prelude traits follow the 2018 rules.
use std::any::Any;
use std::panic;

fn payload(p: Box<dyn Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        format!("&str {:?}", s)
    } else if let Some(s) = p.downcast_ref::<String>() {
        format!("String {:?}", s)
    } else if let Some(n) = p.downcast_ref::<i32>() {
        format!("i32 {}", n)
    } else {
        "other".to_string()
    }
}

fn catch<F: FnOnce() + panic::UnwindSafe>(f: F) -> String {
    payload(panic::catch_unwind(f).unwrap_err())
}

macro_rules! cstr_split {
    ($i:ident $s:literal) => { concat!(stringify!($i), "+", $s) };
    ($s:literal) => { "one literal" };
}

macro_rules! alts {
    ($a:pat | $b:pat) => { "two" };
    ($a:pat) => { "one" };
}

fn checked(a: u32, b: u32) -> Result<u32, String> {
    let s = r#try!(a.checked_add(b).ok_or_else(|| "overflow".to_string()));
    Ok(s)
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    println!("{}", catch(|| panic!("{}")));
    println!("{}", catch(|| panic!("50% {done}",)));
    println!("{}", catch(|| panic!("{} and {}", 1, 2)));
    println!("{}", catch(|| panic!(String::from("owned {}"))));
    println!("{}", catch(|| panic!(42)));
    println!("{}", catch(|| panic!(concat!("a", "{}"))));
    println!("{}", catch(|| assert!(1 > 2, "{} not formatted")));
    println!("{}", catch(|| assert!(1 > 2, "{} formatted", 7)));
    println!("{}", catch(|| debug_assert!(false, "x{{y")));
    println!("{}", catch(|| unreachable!("{}")));
    println!("{}", catch(|| unreachable!("{}", 3)));
    println!("{}", catch(|| todo!("{{}}")));
    let _ = panic::take_hook();

    let arr = [1u8, 2, 3];
    let sizes: Vec<usize> = arr.into_iter().map(|x| std::mem::size_of_val(&x)).collect();
    println!("array.into_iter: {:?}", sizes);
    let sizes: Vec<usize> = IntoIterator::into_iter(arr).map(|x| std::mem::size_of_val(&x)).collect();
    println!("IntoIterator::into_iter(array): {:?}", sizes);
    let mut by_for = Vec::new();
    for x in arr {
        by_for.push(std::mem::size_of_val(&x));
    }
    println!("for x in array: {:?}", by_for);
    let boxed: Box<[u8]> = vec![4u8, 5].into_boxed_slice();
    let sizes: Vec<usize> = boxed.into_iter().map(|x| std::mem::size_of_val(&x)).collect();
    println!("boxed_slice.into_iter: {:?}", sizes);
    let boxed_arr = Box::new([6u8, 7]);
    let v: Vec<u8> = boxed_arr.into_iter().collect();
    println!("boxed_array.into_iter: {:?}", v);

    println!("{} / {}", cstr_split!(c"x"), alts!(1 | 2));
    println!("{:?} {:?}", checked(1, 2), checked(u32::MAX, 1));
}
