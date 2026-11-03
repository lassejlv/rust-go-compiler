// Vec: retain_mut, drain ranges, splice, extend_from_within, split_off, swap_remove, extract_if, chunk ops
fn main() {
    let mut v: Vec<i32> = (1..=12).collect();
    v.retain_mut(|x| {
        *x *= 3;
        *x % 2 == 0
    });
    println!("{:?}", v);
    let d: Vec<i32> = v.drain(1..3).collect();
    println!("drained {:?} rest {:?}", d, v);
    let d2: Vec<i32> = v.drain(..=1).rev().collect();
    println!("{:?} {:?}", d2, v);
    v.extend([100, 200, 300]);
    let tail: Vec<i32> = v.drain(2..).collect();
    println!("{:?} {:?}", v, tail);

    let mut s = vec!['a', 'b', 'c', 'd', 'e'];
    let removed: Vec<char> = s.splice(1..3, ['X', 'Y', 'Z']).collect();
    println!("{:?} removed {:?}", s, removed);
    let removed2: String = s.splice(..2, std::iter::empty()).collect();
    println!("{:?} removed {:?}", s, removed2);
    s.splice(s.len().., "end".chars());
    println!("{:?}", s);
    s.splice(0..0, ['^']);
    println!("{}", s.iter().collect::<String>());

    let mut e = vec![1, 2, 3];
    e.extend_from_within(..2);
    e.extend_from_within(1..);
    println!("{:?}", e);
    let mut st = vec![String::from("x"), String::from("y")];
    st.extend_from_within(..);
    st[0].push('!');
    println!("{:?}", st);
    let mut w = vec![10, 20, 30, 40, 50];
    let back = w.split_off(3);
    println!("{:?} {:?}", w, back);
    let sr = w.swap_remove(0);
    println!("{} {:?}", sr, w);
    w.insert(1, 15);
    w.truncate(2);
    println!("{:?}", w);
    let mut ex: Vec<i32> = (0..10).collect();
    let evens: Vec<i32> = ex.extract_if(.., |x| *x % 2 == 0).collect();
    println!("{:?} {:?}", evens, ex);
    let big: Vec<i32> = ex.extract_if(1..4, |x| *x > 2).collect();
    println!("{:?} {:?}", big, ex);
    let mut r = vec![1, 2, 3, 4, 5, 6, 7];
    r.rotate_left(2);
    println!("{:?}", r);
    r[1..5].rotate_right(1);
    println!("{:?}", r);
    r.reverse();
    r[..3].reverse();
    println!("{:?}", r);
    let (left, right) = r.split_at_mut(3);
    left[0] += right[0];
    right.swap(0, 1);
    println!("{:?}", r);
    r.copy_within(0..2, 5);
    println!("{:?}", r);
    r.fill(0);
    r[2..4].fill_with(|| 9);
    println!("{:?}", r);
    let mut dd = vec![1, 1, 1, 2, 2, 1, 3, 3];
    dd.dedup();
    println!("{:?}", dd);
    let mut nested = vec![vec![1], vec![2, 3]];
    nested.iter_mut().for_each(|v| v.push(0));
    let flat = nested.concat();
    println!("{:?} {:?}", flat, nested.join(&-1));
    let mut cap = Vec::with_capacity(10);
    cap.push(1u8);
    println!("{} {}", cap.len(), cap.capacity() >= 10);
    cap.shrink_to_fit();
    cap.reserve(5);
    println!("{}", cap.capacity() >= 6);
    let mut p = vec![3, 1, 2];
    if let Some(last) = p.last_mut() {
        *last = 99;
    }
    if let Some((first, rest)) = p.split_first_mut() {
        *first += rest.iter().sum::<i32>();
    }
    println!("{:?} {:?}", p, p.split_last());
    let mut q = vec![5, 6, 7, 8];
    while let Some(x) = q.pop_if(|x| *x > 6) {
        print!("{} ", x);
    }
    println!("{:?}", q);
    let mut stack = vec![1];
    stack.append(&mut vec![2, 3]);
    let mut other = vec![4];
    stack.append(&mut other);
    println!("{:?} {:?}", stack, other);
    let sl = &stack[..];
    println!("{:?} {:?}", sl.first().zip(sl.last()), sl.iter().rev().step_by(2).collect::<Vec<_>>());
    let mut v2 = vec![0; 3];
    v2.resize_with(6, Default::default);
    v2.clear();
    println!("{} {:?}", v2.is_empty(), v2);
    let repeated = [1, 2].repeat(3);
    println!("{:?}", repeated);
}
