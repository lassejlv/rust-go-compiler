fn sum(arr: [i32; 5]) -> i32 {
    let mut total = 0;
    for i in 0..5 {
        total += arr[i];
    }
    total
}

fn main() {
    let a = [3, 1, 4, 1, 5];
    println!("a[0] = {}, a[4] = {}", a[0], a[4]);
    println!("len = {}", a.len());
    println!("sum = {}", sum(a));
    let mut b = [0; 8];
    for i in 0..b.len() {
        b[i] = i * i;
    }
    println!("b = {:?}", b);
    b[3] = 100;
    println!("b[3] = {}", b[3]);
    for x in a.iter() {
        print!("{} ", x);
    }
    println!();
    for x in a {
        print!("{} ", x * 10);
    }
    println!();
    let grid = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    let mut diag = 0;
    for i in 0..3 {
        diag += grid[i][i];
    }
    println!("diag = {}", diag);
    println!("grid = {:?}", grid);
    let mut max = a[0];
    for &x in a.iter() {
        if x > max {
            max = x;
        }
    }
    println!("max = {}", max);
    println!("contains 4: {}", a.contains(&4));
    let strs = ["alpha", "beta", "gamma"];
    for (i, s) in strs.iter().enumerate() {
        println!("{}: {}", i, s);
    }
    let copy = a;
    println!("copy == a: {}", copy == a);
    let mut sorted = a;
    sorted.sort();
    println!("sorted = {:?}, original = {:?}", sorted, a);
    let bools = [true; 3];
    println!("bools = {:?}", bools);
}
