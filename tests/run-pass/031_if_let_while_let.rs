enum Token {
    Number(i32),
    Word(String),
    End,
}

fn main() {
    let config: Option<u32> = Some(8);
    if let Some(threads) = config {
        println!("using {} threads", threads);
    } else {
        println!("default threads");
    }
    let none: Option<u32> = None;
    if let Some(v) = none {
        println!("unexpected {}", v);
    } else {
        println!("none branch");
    }

    let mut stack = vec![1, 2, 3, 4];
    while let Some(top) = stack.pop() {
        println!("popped {}", top);
    }
    println!("stack empty: {}", stack.is_empty());

    let tokens = vec![
        Token::Number(5),
        Token::Word("apple".to_string()),
        Token::Number(-2),
        Token::End,
        Token::Number(100),
    ];
    let mut i = 0;
    while let Some(tok) = tokens.get(i) {
        if let Token::End = tok {
            println!("end token at {}", i);
            break;
        }
        if let Token::Number(n) = tok {
            println!("number {}", n);
        } else if let Token::Word(w) = tok {
            println!("word {}", w);
        }
        i += 1;
    }

    let mut n = 37u32;
    let mut chain = Vec::new();
    while let Some(next) = if n == 1 { None } else { Some(if n % 2 == 0 { n / 2 } else { 3 * n + 1 }) } {
        chain.push(next);
        n = next;
    }
    println!("chain len = {}, last few = {:?}", chain.len(), &chain[chain.len() - 4..]);

    let pairs = vec![(1, 'a'), (2, 'b'), (3, 'c')];
    let mut it = pairs.iter();
    while let Some(&(num, ch)) = it.next() {
        print!("{}{} ", num, ch);
    }
    println!();

    let value: Result<i32, String> = Ok(7);
    if let Ok(v) = value {
        if v > 5 {
            println!("ok and big: {}", v);
        }
    }
    let x = Some(3);
    if let Some(v @ 3..=10) = x {
        println!("binding with range: {}", v);
    }
}
