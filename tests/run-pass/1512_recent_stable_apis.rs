// Recently stabilized std APIs that were missing: array_windows, element_offset,
// strip_circumfix, get_disjoint_unchecked_mut, LazyCell/LazyLock::force_mut,
// String::from_utf16le/be(_lossy), Vec/VecDeque/LinkedList push_mut & co,
// Vec/String::into_raw_parts, Peekable::next_if_map(_mut),
// Duration::from_nanos_u128, ControlFlow::break_ok/continue_ok,
// Result::unwrap_err_unchecked, f64::algebraic_*/to_int_unchecked,
// LinkedList::extract_if.
use std::cell::LazyCell;
use std::collections::{LinkedList, VecDeque};
use std::ops::ControlFlow;
use std::sync::LazyLock;
use std::time::Duration;

fn main() {
    let v = [1, 2, 3, 4, 5];
    println!("{:?}", v.array_windows::<2>().collect::<Vec<_>>());
    println!("{:?}", v.array_windows::<3>().rev().map(|[a, b, c]| a + b + c).collect::<Vec<_>>());
    println!("{} {:?}", v.array_windows::<4>().len(), v.array_windows::<9>().next());
    println!("{:?} {:?}", v.element_offset(&v[3]), v.element_offset(&0));
    println!("{:?} {:?}", v.strip_circumfix(&[1], &[5]), v.strip_circumfix(&[2], &[5]));
    println!("{:?} {:?}", "[x]".strip_circumfix('[', ']'), "(x".strip_circumfix("(", ")"));
    let mut w = vec![10, 20, 30];
    let [a, b] = unsafe { w.get_disjoint_unchecked_mut([0, 2]) };
    std::mem::swap(a, b);
    println!("{:?}", w);

    let mut lc = LazyCell::new(|| vec![1]);
    LazyCell::force_mut(&mut lc).push(2);
    println!("{:?}", *lc);
    let mut ll: LazyLock<String> = LazyLock::new(|| "lazy".to_string());
    LazyLock::force_mut(&mut ll).push('!');
    println!("{}", *ll);

    println!("{:?}", String::from_utf16le(&[0x68, 0, 0x69, 0]));
    println!("{:?}", String::from_utf16be(&[0, 0x68, 0, 0x69]));
    let odd = String::from_utf16le(&[0x68, 0, 0x69]).unwrap_err();
    println!("{} / {:?}", odd, odd);
    println!("{:?}", String::from_utf16le(&[0x00, 0xD8]).map_err(|e| e.to_string()));
    println!("{:?} {:?}", String::from_utf16le_lossy(&[0x68, 0, 0x69]), String::from_utf16be_lossy(&[0xD8, 0, 0, 0x61]));

    let mut vv = vec![1];
    *vv.push_mut(2) += 10;
    *vv.insert_mut(0, 0) -= 5;
    println!("{:?}", vv);
    let mut dq = VecDeque::from([5]);
    *dq.push_back_mut(6) *= 2;
    *dq.push_front_mut(4) *= 3;
    *dq.insert_mut(1, 0) = 99;
    println!("{:?}", dq);
    let mut list: LinkedList<i32> = (1..=6).collect();
    *list.push_back_mut(7) += 100;
    *list.push_front_mut(0) -= 1;
    let evens: Vec<i32> = list.extract_if(|x| *x % 2 == 0).collect();
    println!("{:?} {:?}", evens, list);

    let (p, len, cap) = vec![7u32, 8, 9].into_raw_parts();
    let back = unsafe { Vec::from_raw_parts(p, len, cap) };
    println!("{:?} {}", back, len);
    let (p, len, cap) = String::from("raw").into_raw_parts();
    println!("{}", unsafe { String::from_raw_parts(p, len, cap) });

    let mut it = "123abc".chars().peekable();
    let mut n = 0;
    while let Some(d) = it.next_if_map(|c| c.to_digit(10).ok_or(c)) {
        n = n * 10 + d;
    }
    println!("{} {:?}", n, it.next());
    let mut it2 = [1, 2, 3].into_iter().peekable();
    println!("{:?} {:?} {:?}", it2.next_if_map_mut(|x| { *x += 1; None::<i32> }), it2.next(), it2.next_if_map_mut(|x| Some(*x * 10)));

    println!("{:?}", Duration::from_nanos_u128(3_000_000_001));
    let c: ControlFlow<&str, i32> = ControlFlow::Break("b");
    println!("{:?} {:?}", c.break_ok(), c.continue_ok());
    println!("{:?}", ControlFlow::<&str, i32>::Continue(1).break_ok());
    println!("{}", unsafe { Err::<i32, &str>("e").unwrap_err_unchecked() });
    println!("{} {}", 1.5f64.algebraic_mul(2.0), 7.0f32.algebraic_rem(4.0));
    println!("{} {}", unsafe { 2.9f64.to_int_unchecked::<u8>() }, unsafe { (-3.7f32).to_int_unchecked::<i64>() });
}
