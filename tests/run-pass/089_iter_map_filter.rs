fn main() {
    let v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    println!("doubled: {:?}", doubled);
    let evens: Vec<i32> = v.iter().filter(|&&x| x % 2 == 0).cloned().collect();
    println!("evens: {:?}", evens);
    let odd_squares: Vec<i32> = v.iter().filter(|x| *x % 2 == 1).map(|x| x * x).collect();
    println!("odd squares: {:?}", odd_squares);
    let strs = vec!["3", "x", "10", "-2", "4.5", "7"];
    let parsed: Vec<i32> = strs.iter().filter_map(|s| s.parse().ok()).collect();
    println!("filter_map parsed: {:?}", parsed);
    for (i, s) in strs.iter().enumerate().filter(|(i, _)| i % 2 == 0) {
        println!("  even index {}: {}", i, s);
    }
    let indexed: Vec<String> = v.iter().enumerate().map(|(i, x)| format!("{}:{}", i, x)).take(4).collect();
    println!("indexed: {:?}", indexed);
    let words = vec!["apple", "bob", "kayak", "rust", "level"];
    let palins: Vec<&&str> = words.iter().filter(|w| w.chars().rev().collect::<String>() == **w).collect();
    println!("palindromes: {:?}", palins);
    let lens: Vec<usize> = words.iter().map(|w| w.len()).collect();
    println!("lens: {:?}", lens);
    let upper: Vec<String> = words.iter().map(|w| w.to_uppercase()).filter(|w| w.contains('A')).collect();
    println!("upper with A: {:?}", upper);
    let owned: Vec<String> = vec!["a".to_string(), "bb".to_string()];
    let moved: Vec<usize> = owned.into_iter().map(|s| s.len()).collect();
    println!("into_iter map: {:?}", moved);
    let nested: Vec<i32> = v.iter().map(|x| x * 10).filter(|x| x % 30 != 0).map(|x| x + 1).collect();
    println!("chain of maps: {:?}", nested);
    let first_big = v.iter().map(|x| x * x).find(|&x| x > 40);
    println!("first square > 40: {:?}", first_big);
    let opts = vec![Some(1), None, Some(3), None, Some(5)];
    let somes: Vec<i32> = opts.iter().flatten().cloned().collect();
    println!("flatten options: {:?}", somes);
    let map_while: Vec<i32> = strs.iter().map_while(|s| s.parse::<i32>().ok()).collect();
    println!("map_while: {:?}", map_while);
    let copied: Vec<i32> = v.iter().copied().filter(|x| x > &7).collect();
    println!("copied: {:?}", copied);
}
