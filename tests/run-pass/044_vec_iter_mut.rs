fn main() {
    let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8];
    for x in v.iter_mut() {
        *x *= 3;
    }
    println!("tripled: {:?}", v);
    for x in &mut v {
        *x -= 1;
    }
    println!("minus one: {:?}", v);
    v.retain(|&x| x % 2 == 0);
    println!("retain even: {:?}", v);
    v.extend(vec![100, 200]);
    v.extend([1, 2].iter());
    v.extend_from_slice(&[7, 7]);
    println!("extended: {:?}", v);
    v.swap(0, 1);
    println!("swapped: {:?}", v);
    v.truncate(6);
    println!("truncated: {:?}", v);
    let drained: Vec<i32> = v.drain(1..3).collect();
    println!("drained {:?} rest {:?}", drained, v);
    let mut owned_strings = vec![String::from("a"), String::from("b")];
    for s in owned_strings.iter_mut() {
        s.push('!');
    }
    println!("strings: {:?}", owned_strings);
    let mut total_len = 0;
    for s in owned_strings.into_iter() {
        total_len += s.len();
    }
    println!("total len: {}", total_len);
    let mut a = vec![1, 2, 3];
    let mut b = vec![4, 5];
    a.append(&mut b);
    println!("append: a={:?} b={:?}", a, b);
    let tail = a.split_off(3);
    println!("split_off: a={:?} tail={:?}", a, tail);
    let mut grid = vec![0; 6];
    for (i, cell) in grid.iter_mut().enumerate() {
        *cell = i * 10;
    }
    println!("grid: {:?}", grid);
    let mut nums = vec![3, 1, 2];
    nums.iter_mut().for_each(|n| *n = *n * *n);
    println!("for_each: {:?}", nums);
    let mut stack = Vec::new();
    for i in 0..4 {
        stack.push(i);
    }
    while let Some(top) = stack.pop() {
        if top == 1 {
            break;
        }
        print!("{} ", top);
    }
    println!("| left: {:?}", stack);
    let mut v2 = vec![5, 6, 7];
    v2.resize(5, 0);
    println!("resize up: {:?}", v2);
    v2.resize(2, 0);
    println!("resize down: {:?}", v2);
    v2.fill(9);
    println!("fill: {:?}", v2);
    let mut rot = vec![1, 2, 3, 4, 5];
    rot.rotate_left(2);
    println!("rotate_left: {:?}", rot);
    rot.rotate_right(1);
    println!("rotate_right: {:?}", rot);
}
