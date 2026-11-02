fn main() {
    let sentences = vec!["the cat", "a quick dog", "hi"];
    let words: Vec<&str> = sentences.iter().flat_map(|s| s.split(' ')).collect();
    println!("flat_map words: {:?}", words);
    let nested = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let flat: Vec<i32> = nested.iter().flatten().cloned().collect();
    println!("flatten: {:?}", flat);
    let owned_flat: Vec<i32> = nested.into_iter().flatten().collect();
    println!("owned flatten: {:?}", owned_flat);
    let pairs: Vec<(i32, i32)> = (1..=3).flat_map(|i| (i..=3).map(move |j| (i, j))).collect();
    println!("pairs i<=j: {:?}", pairs);
    let sum = (1..=10).fold(0, |acc, x| acc + x);
    println!("fold sum: {}", sum);
    let product: u64 = (1..=15u64).fold(1, |acc, x| acc * x);
    println!("fold product: {}", product);
    let s = ["a", "b", "c"].iter().fold(String::new(), |mut acc, x| {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(x);
        acc
    });
    println!("fold string: {}", s);
    let rev = (1..=5).fold(Vec::new(), |mut v, x| {
        v.insert(0, x);
        v
    });
    println!("fold reverse: {:?}", rev);
    let max = [3, 9, 2, 7].iter().fold(i32::MIN, |m, &x| m.max(x));
    println!("fold max: {}", max);
    let r = (1..=5).reduce(|a, b| a * b);
    println!("reduce product: {:?}", r);
    let empty: Option<i32> = std::iter::empty::<i32>().reduce(|a, b| a + b);
    println!("reduce empty: {:?}", empty);
    let prefix: Vec<i32> = [1, 2, 3, 4].iter().scan(0, |st, &x| {
        *st += x;
        Some(*st)
    }).collect();
    println!("scan prefix sums: {:?}", prefix);
    let chars: Vec<char> = vec!["ab", "cd"].iter().flat_map(|s| s.chars()).collect();
    println!("flat_map chars: {:?}", chars);
    let try_sum: Option<i32> = [1, 2, 3].iter().try_fold(0i32, |acc, &x| acc.checked_add(x));
    println!("try_fold: {:?}", try_sum);
    let overflow: Option<i8> = [100i8, 27, 1].iter().try_fold(0i8, |acc, &x| acc.checked_add(x));
    println!("try_fold overflow: {:?}", overflow);
    let matrix = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let total: i32 = matrix.iter().map(|row| row.iter().sum::<i32>()).sum();
    println!("matrix total: {}", total);
    let counts = "hello world".chars().fold([0usize; 2], |mut c, ch| {
        if "aeiou".contains(ch) {
            c[0] += 1;
        } else if ch.is_alphabetic() {
            c[1] += 1;
        }
        c
    });
    println!("vowels={} consonants={}", counts[0], counts[1]);
}
