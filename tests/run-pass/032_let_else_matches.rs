#[derive(Debug)]
enum Cmd {
    Push(i32),
    Pop,
    Add,
    Print,
}

fn parse_cmd(line: &str) -> Option<Cmd> {
    let mut parts = line.split_whitespace();
    let Some(op) = parts.next() else {
        return None;
    };
    match op {
        "push" => {
            let Some(arg) = parts.next() else { return None; };
            let Ok(n) = arg.parse::<i32>() else {
                println!("  bad number: {}", arg);
                return None;
            };
            Some(Cmd::Push(n))
        }
        "pop" => Some(Cmd::Pop),
        "add" => Some(Cmd::Add),
        "print" => Some(Cmd::Print),
        _ => None,
    }
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')
}

fn main() {
    let program = ["push 3", "push 4", "add", "print", "push x", "", "jump 5", "push 10", "pop", "print"];
    let mut stack: Vec<i32> = Vec::new();
    for line in program {
        let Some(cmd) = parse_cmd(line) else {
            println!("skip {:?}", line);
            continue;
        };
        println!("exec {:?}", cmd);
        match cmd {
            Cmd::Push(n) => stack.push(n),
            Cmd::Pop => {
                stack.pop();
            }
            Cmd::Add => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a + b);
            }
            Cmd::Print => println!("  stack = {:?}", stack),
        }
    }
    let vowels = "Programming in Rust".chars().filter(|&c| is_vowel(c)).count();
    println!("vowels = {}", vowels);
    let cmds = [Cmd::Push(1), Cmd::Pop, Cmd::Push(-5), Cmd::Print];
    let pushes = cmds.iter().filter(|c| matches!(c, Cmd::Push(_))).count();
    let neg_pushes = cmds.iter().filter(|c| matches!(c, Cmd::Push(n) if *n < 0)).count();
    println!("pushes = {}, negative pushes = {}", pushes, neg_pushes);
    println!("matches range: {} {}", matches!(42, 1..=50), matches!(Some(3), Some(4..)));
}
