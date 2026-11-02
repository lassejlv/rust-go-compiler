#[derive(Debug, Clone)]
struct Student {
    name: String,
    grade: u32,
    age: u32,
}

fn main() {
    let mut v = vec![5, 3, 8, 1, 9, 2, 7, 3, 5, 5];
    v.sort();
    println!("sorted: {:?}", v);
    v.dedup();
    println!("dedup: {:?}", v);
    v.reverse();
    println!("reversed: {:?}", v);
    v.sort_by(|a, b| b.cmp(a));
    println!("sort desc: {:?}", v);
    let mut w = vec![-4, 2, -1, 3, 0, -3];
    w.sort_by_key(|x: &i32| x.abs());
    println!("sort_by_key abs: {:?}", w);
    w.sort_unstable();
    println!("sort_unstable: {:?}", w);
    println!("binary_search 2: {:?}", w.binary_search(&2));
    println!("binary_search 1: {:?}", w.binary_search(&1));
    println!("binary_search -10: {:?}", w.binary_search(&-10));
    let mut fl = vec![2.5, -1.0, 3.75, 0.0];
    fl.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("floats: {:?}", fl);
    let mut students = vec![
        Student { name: "Zoe".to_string(), grade: 90, age: 20 },
        Student { name: "Adam".to_string(), grade: 85, age: 22 },
        Student { name: "Mia".to_string(), grade: 90, age: 19 },
        Student { name: "Bob".to_string(), grade: 70, age: 22 },
    ];
    students.sort_by(|a, b| a.name.cmp(&b.name));
    let names: Vec<&str> = students.iter().map(|s| s.name.as_str()).collect();
    println!("by name: {:?}", names);
    students.sort_by(|a, b| b.grade.cmp(&a.grade).then(a.age.cmp(&b.age)));
    for s in &students {
        println!("  {} grade={} age={}", s.name, s.grade, s.age);
    }
    students.sort_by_key(|s| (s.age, s.name.clone()));
    let names: Vec<String> = students.iter().map(|s| format!("{}({})", s.name, s.age)).collect();
    println!("by age then name: {}", names.join(" "));
    let mut words = vec!["banana", "Apple", "cherry", "apple", "Banana"];
    words.sort();
    println!("words: {:?}", words);
    words.sort_by_key(|w| w.len());
    println!("by len (stable): {:?}", words);
    let mut d = vec![1, 1, 2, 2, 2, 3, 1, 1];
    d.dedup();
    println!("dedup consecutive: {:?}", d);
    let mut e = vec![10, 11, 20, 21, 22, 30];
    e.dedup_by_key(|x| *x / 10);
    println!("dedup_by_key: {:?}", e);
    let is_sorted = e.windows(2).all(|p| p[0] <= p[1]);
    println!("is sorted: {}", is_sorted);
    let mut chars: Vec<char> = "sorting".chars().collect();
    chars.sort();
    println!("chars sorted: {}", chars.iter().collect::<String>());
}
