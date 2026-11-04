// Panic messages of std methods (printed by a hook): integer log base,
// repeat capacity overflow, VecDeque::swap asserts, BTreeMap::range on an
// empty map (no panic), plus overflow/Duration/slice/str/RefCell messages.
use std::panic;
fn main() {
    panic::set_hook(Box::new(|i| {
        let m = i.payload().downcast_ref::<&str>().map(|s| s.to_string()).or(i.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        println!("panic: {}", m);
    }));

    let _ = panic::catch_unwind(|| std::hint::black_box(i32::MIN).abs());
    let _ = panic::catch_unwind(|| std::hint::black_box(3i32).pow(40));
    let _ = panic::catch_unwind(|| std::hint::black_box(-4i32).isqrt());
    let _ = panic::catch_unwind(|| std::hint::black_box(0u32).ilog2());
    let _ = panic::catch_unwind(|| std::hint::black_box(0u32).ilog10());
    let _ = panic::catch_unwind(|| std::hint::black_box(5u32).ilog(1));
    let _ = panic::catch_unwind(|| std::hint::black_box(5i32).div_euclid(0));
    let _ = panic::catch_unwind(|| std::hint::black_box(5i32).rem_euclid(0));
    let _ = panic::catch_unwind(|| std::hint::black_box(u32::MAX).next_power_of_two());
    let _ = panic::catch_unwind(|| std::hint::black_box(7u32).div_ceil(0));
    let _ = panic::catch_unwind(|| std::hint::black_box(7u32).next_multiple_of(0));
    let _ = panic::catch_unwind(|| std::hint::black_box(i64::MIN) / -1);
    let _ = panic::catch_unwind(|| std::hint::black_box(i64::MIN) % -1);
    let _ = panic::catch_unwind(|| -std::hint::black_box(i8::MIN));
    let _ = panic::catch_unwind(|| std::hint::black_box(1u8) << std::hint::black_box(9));
    let _ = panic::catch_unwind(|| std::hint::black_box(200u8) as i8 as u8 + 100);
    let _ = panic::catch_unwind(|| std::hint::black_box(5usize) - 6);
    let _ = panic::catch_unwind(|| std::hint::black_box(u64::MAX) * 2);
    let _ = panic::catch_unwind(|| std::hint::black_box(i32::MIN).unsigned_abs());
    let _ = panic::catch_unwind(|| std::hint::black_box(2.5f64).clamp(3.0, 1.0));
    let _ = panic::catch_unwind(|| std::hint::black_box(5).clamp(3, 1));
    let _ = panic::catch_unwind(|| char::from_digit(1, 40));
    let _ = panic::catch_unwind(|| 'a'.to_digit(37));
    let _ = panic::catch_unwind(|| u32::from_str_radix("1", 1));
    let _ = panic::catch_unwind(|| std::time::Duration::from_secs(u64::MAX) + std::time::Duration::from_secs(1));
    let _ = panic::catch_unwind(|| std::time::Duration::from_secs(1) - std::time::Duration::from_secs(2));
    let _ = panic::catch_unwind(|| std::time::Duration::from_secs(1) / 0);
    let _ = panic::catch_unwind(|| std::time::Duration::from_secs_f64(-1.0));
    let _ = panic::catch_unwind(|| std::time::Duration::from_secs_f64(f64::NAN));
    let _ = panic::catch_unwind(|| std::time::Duration::new(u64::MAX, 2_000_000_000));
    let _ = panic::catch_unwind(|| { vec![1, 2].split_at(3).0.len() });
    let _ = panic::catch_unwind(|| "abc".split_at(5));
    let _ = panic::catch_unwind(|| [1, 2, 3].chunks(0).count());
    let _ = panic::catch_unwind(|| [1, 2, 3].windows(0).count());
    let _ = panic::catch_unwind(|| vec![1].swap_remove(3));
    let _ = panic::catch_unwind(|| vec![1].insert(3, 1));
    let _ = panic::catch_unwind(|| { vec![1].drain(2..).count() });
    let _ = panic::catch_unwind(|| String::from("é").truncate(1));
    let _ = panic::catch_unwind(|| String::from("é").insert(1, 'x'));
    let _ = panic::catch_unwind(|| [1, 2].rotate_left(3));
    let _ = panic::catch_unwind(|| [0u8; 3].copy_within(1..5, 0));
    let _ = panic::catch_unwind(|| vec![0u8; 2].repeat(usize::MAX));
    let _ = panic::catch_unwind(|| "x".repeat(usize::MAX));
    let _ = panic::catch_unwind(|| std::iter::repeat(1).step_by(0).next());
    let _ = panic::catch_unwind(|| (0..10).step_by(0).next());
    let _ = panic::catch_unwind(|| std::collections::VecDeque::<i32>::new().swap(0, 1));
    let _ = panic::catch_unwind(|| std::collections::BTreeMap::<i32, i32>::new().range(5..1).count());
    let _ = panic::catch_unwind(|| std::collections::BTreeMap::<i32, i32>::new().range((std::ops::Bound::Excluded(1), std::ops::Bound::Excluded(1))).count());
    let _ = panic::catch_unwind(|| std::cell::RefCell::new(1).borrow_mut().clone_from(&2));
    let _ = panic::catch_unwind(|| Option::<i32>::None.unwrap());
    let _ = panic::catch_unwind(|| Err::<i32, String>("e".into()).expect("ctx"));
    let _ = panic::catch_unwind(|| Ok::<i32, String>(1).unwrap_err());
    let _ = panic::catch_unwind(|| Some(1).expect_none_compat());
}
trait EN { fn expect_none_compat(self) -> i32; }
impl EN for Option<i32> { fn expect_none_compat(self) -> i32 { 0 } }
