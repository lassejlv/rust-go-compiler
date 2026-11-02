fn main() {
    let add = |a: i32, b: i32| a + b;
    let square = |x| x * x;
    let greet = || println!("hello from closure");
    println!("add = {}", add(2, 3));
    println!("square = {}", square(7));
    greet();

    let factor = 3;
    let scale = |x: i32| x * factor;
    println!("scale = {}", scale(5));
    println!("factor still usable = {}", factor);

    let mut count = 0;
    let mut inc = || {
        count += 1;
        println!("  inc -> {}", count);
    };
    inc();
    inc();
    inc();
    println!("count = {}", count);

    let name = String::from("Ferris");
    let consume = move || format!("moved {}", name);
    println!("{}", consume());
    println!("{}", consume());

    let mut v = vec![1, 2, 3];
    let mut push = |x| v.push(x);
    push(4);
    push(5);
    println!("v = {:?}", v);

    let block = |x: i32| -> i32 {
        let y = x + 1;
        y * y
    };
    println!("block = {}", block(4));

    let compose = |x: i32| scale(add(x, 1));
    println!("compose = {}", compose(1));

    let nums = vec![5, 3, 8];
    let threshold = 4;
    let above: Vec<&i32> = nums.iter().filter(|&&n| n > threshold).collect();
    println!("above = {:?}", above);

    let mut total = 0;
    nums.iter().for_each(|n| total += n);
    println!("total = {}", total);

    let s = String::from("borrowed");
    let len_of = || s.len();
    println!("len = {} and s = {}", len_of(), s);

    let make_pair = |a, b| (b, a);
    println!("pair = {:?}", make_pair("x", 1.5));

    let fns = [|x: i32| x + 1, |x: i32| x * 2];
    println!("fn array = {} {}", fns[0](10), fns[1](10));

    let mut state = 1u64;
    let mut next_rand = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) % 100
    };
    let r: Vec<u64> = (0..5).map(|_| next_rand()).collect();
    println!("lcg = {:?}", r);
}
