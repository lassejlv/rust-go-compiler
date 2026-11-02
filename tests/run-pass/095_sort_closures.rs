#[derive(Debug, Clone)]
struct Movie {
    title: String,
    year: u32,
    rating: f32,
    genre: &'static str,
}

fn movie(t: &str, y: u32, r: f32, g: &'static str) -> Movie {
    Movie { title: t.to_string(), year: y, rating: r, genre: g }
}

fn main() {
    let movies = vec![
        movie("Alien", 1979, 8.5, "scifi"),
        movie("Heat", 1995, 8.3, "crime"),
        movie("Arrival", 2016, 7.9, "scifi"),
        movie("Fargo", 1996, 8.1, "crime"),
        movie("Up", 2009, 8.3, "family"),
        movie("Coco", 2017, 8.4, "family"),
        movie("Dune", 2021, 8.0, "scifi"),
    ];
    let mut by_year = movies.clone();
    by_year.sort_by_key(|m| m.year);
    println!("by year: {:?}", by_year.iter().map(|m| m.year).collect::<Vec<_>>());
    let mut by_rating = movies.clone();
    by_rating.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap().then_with(|| a.title.cmp(&b.title)));
    for m in &by_rating {
        println!("  {:<8} {:.1}", m.title, m.rating);
    }
    let mut by_genre = movies.clone();
    by_genre.sort_by(|a, b| a.genre.cmp(b.genre).then(b.year.cmp(&a.year)));
    for m in &by_genre {
        println!("  {:<7} {} {}", m.genre, m.year, m.title);
    }
    let mut titles: Vec<&str> = movies.iter().map(|m| m.title.as_str()).collect();
    titles.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| b.cmp(a)));
    println!("titles by len then reverse alpha: {:?}", titles);
    let mut cached = movies.clone();
    cached.sort_by_cached_key(|m| m.title.to_lowercase().chars().rev().collect::<String>());
    println!("by reversed title: {:?}", cached.iter().map(|m| &m.title).collect::<Vec<_>>());
    let mut nums = vec![-5i32, 3, -1, 4, -2, 0];
    nums.sort_by_key(|&x| (x < 0, x.abs()));
    println!("non-negatives first by abs: {:?}", nums);
    let mut floats = vec![2.5f64, -0.5, 10.0, 3.25];
    floats.sort_by(|a, b| b.total_cmp(a));
    println!("floats desc: {:?}", floats);
    let best_per_genre: Vec<(&str, &str)> = ["crime", "family", "scifi"]
        .iter()
        .map(|g| {
            let best = movies
                .iter()
                .filter(|m| m.genre == *g)
                .max_by(|a, b| a.rating.partial_cmp(&b.rating).unwrap())
                .unwrap();
            (*g, best.title.as_str())
        })
        .collect();
    println!("best per genre: {:?}", best_per_genre);
    let mut idx: Vec<usize> = (0..movies.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(movies[i].year));
    println!("argsort by year desc: {:?}", idx);
    let mut v = vec![5, 1, 4, 2, 3];
    v.sort_unstable_by(|a, b| (a % 2).cmp(&(b % 2)).then(a.cmp(b)));
    println!("evens then odds: {:?}", v);
    let k = 3;
    let mut top = movies.iter().map(|m| (m.rating * 10.0) as u32).collect::<Vec<_>>();
    top.sort_unstable_by(|a, b| b.cmp(a));
    top.truncate(k);
    println!("top {} ratings x10: {:?}", k, top);
}
