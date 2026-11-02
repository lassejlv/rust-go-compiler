fn add_one(x: &mut i32) {
    *x += 1;
}

fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() >= b.len() { a } else { b }
}

fn sum_ref(v: &Vec<i32>) -> i32 {
    let mut s = 0;
    for x in v {
        s += x;
    }
    s
}

fn push_squares(v: &mut Vec<i32>, n: i32) {
    for i in 1..=n {
        v.push(i * i);
    }
}

fn swap_vals(a: &mut i32, b: &mut i32) {
    let t = *a;
    *a = *b;
    *b = t;
}

struct Account {
    balance: i64,
}

fn deposit(acc: &mut Account, amount: i64) {
    acc.balance += amount;
}

fn main() {
    let mut n = 5;
    add_one(&mut n);
    add_one(&mut n);
    println!("n = {}", n);
    {
        let r = &mut n;
        *r *= 10;
    }
    println!("n = {}", n);
    let r1 = &n;
    let r2 = &n;
    println!("r1 = {}, r2 = {}, sum = {}", r1, r2, *r1 + *r2);
    let rr = &r1;
    println!("double ref = {}", **rr);
    println!("auto deref add = {}", rr.abs() + 1);

    let s1 = String::from("short");
    let s2 = String::from("much longer");
    println!("longest = {}", longest(&s1, &s2));

    let mut v = vec![1, 2, 3];
    push_squares(&mut v, 3);
    println!("v = {:?}, sum = {}", v, sum_ref(&v));

    let mut a = 1;
    let mut b = 2;
    swap_vals(&mut a, &mut b);
    println!("a = {}, b = {}", a, b);

    let mut acc = Account { balance: 100 };
    deposit(&mut acc, 50);
    let bal_ref = &mut acc.balance;
    *bal_ref -= 30;
    println!("balance = {}", acc.balance);

    let mut arr = [1, 2, 3];
    for x in arr.iter_mut() {
        *x = *x * *x + 1;
    }
    println!("arr = {:?}", arr);
    let first = &mut arr[0];
    *first = 99;
    println!("arr = {:?}", arr);

    let pair = (String::from("left"), String::from("right"));
    let (ref l, ref r) = pair;
    println!("refs: {} {} (pair still usable: {})", l, r, pair.0);
    let x = 10;
    let y = &x;
    println!("eq through ref: {}", *y == 10 && y == &10);
}
