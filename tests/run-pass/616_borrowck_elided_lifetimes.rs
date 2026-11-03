// Returning references derived from arguments with lifetime elision.
struct Doc {
    title: String,
    words: Vec<String>,
}

impl Doc {
    fn title(&self) -> &str {
        &self.title
    }
    fn longest(&self) -> &str {
        self.words.iter().max_by_key(|w| w.len()).map(|s| s.as_str()).unwrap_or("")
    }
    fn word_mut(&mut self, i: usize) -> &mut String {
        &mut self.words[i]
    }
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn pick<'a>(a: &'a str, b: &'a str, first: bool) -> &'a str {
    if first { a } else { b }
}

fn main() {
    let mut d = Doc { title: "T".into(), words: vec!["aa".into(), "bbbb".into()] };
    d.word_mut(0).push_str("aaaa");
    let l = d.longest().to_string();
    let t = d.title();
    let owned = String::from("hello world");
    let fw = first_word(&owned);
    println!("{} {} {} {}", t, l, fw, pick(fw, t, false));
}
