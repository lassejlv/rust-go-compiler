fn main() {
    let names = ["ann", "ben", "cat", "dan"];
    let ages = [31, 25, 47];
    for (n, a) in names.iter().zip(ages.iter()) {
        println!("{} is {}", n, a);
    }
    let pairs: Vec<(i32, char)> = (1..).zip("abc".chars()).collect();
    println!("zip with infinite: {:?}", pairs);
    let a = vec![1, 2, 3];
    let b = vec![10, 20];
    let chained: Vec<i32> = a.iter().chain(b.iter()).cloned().collect();
    println!("chain: {:?}", chained);
    let three: Vec<i32> = (1..3).chain(7..9).chain(std::iter::once(100)).collect();
    println!("chain3: {:?}", three);
    let rev: Vec<&str> = names.iter().rev().cloned().collect();
    println!("rev: {:?}", rev);
    println!("take 2: {:?}", names.iter().take(2).collect::<Vec<_>>());
    println!("skip 2: {:?}", names.iter().skip(2).collect::<Vec<_>>());
    let nums = [1, 3, 5, 6, 7, 9, 2];
    let tw: Vec<i32> = nums.iter().cloned().take_while(|x| x % 2 == 1).collect();
    let sw: Vec<i32> = nums.iter().cloned().skip_while(|x| x % 2 == 1).collect();
    println!("take_while odd: {:?}", tw);
    println!("skip_while odd: {:?}", sw);
    let stepped: Vec<i32> = (0..30).step_by(7).collect();
    println!("step_by 7: {:?}", stepped);
    let every_other: Vec<&str> = names.iter().step_by(2).cloned().collect();
    println!("every other: {:?}", every_other);
    let dot: i32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    println!("partial dot: {}", dot);
    let (evens, odds): (Vec<usize>, Vec<usize>) = (0..10).partition(|n| n % 2 == 0);
    println!("partition: {:?} {:?}", evens, odds);
    let (left, right): (Vec<i32>, Vec<char>) = vec![(1, 'a'), (2, 'b')].into_iter().unzip();
    println!("unzip: {:?} {:?}", left, right);
    let rev_enum: Vec<(usize, char)> = "xyz".chars().rev().enumerate().collect();
    println!("rev enumerate: {:?}", rev_enum);
    let enum_rev: Vec<(usize, char)> = "xyz".chars().enumerate().collect::<Vec<_>>().into_iter().rev().collect();
    println!("enumerate rev: {:?}", enum_rev);
    let skip_take: Vec<u32> = (1..=100).skip(10).step_by(10).take(3).collect();
    println!("skip/step/take: {:?}", skip_take);
    let s: String = "hello".chars().zip("WORLD".chars()).flat_map(|(a, b)| [a, b]).collect();
    println!("interleave: {}", s);
    let cyc: Vec<i32> = [1, 2, 3].iter().cycle().take(7).cloned().collect();
    println!("cycle: {:?}", cyc);
}
