#[derive(Debug, Clone)]
struct LogLine {
    ts: u32,
    level: String,
    module: String,
    msg: String,
}

fn parse(line: &str) -> Option<LogLine> {
    let mut parts = line.splitn(4, ' ');
    let ts = parts.next()?.parse().ok()?;
    let level = parts.next()?.trim_matches(|c| c == '[' || c == ']').to_string();
    let module = parts.next()?.trim_end_matches(':').to_string();
    let msg = parts.next()?.to_string();
    Some(LogLine { ts, level, module, msg })
}

fn main() {
    let raw = "\
100 [INFO] net: connected to 10.0.0.1
105 [WARN] db: slow query 230ms
110 [INFO] net: sent 512 bytes
garbage line here
120 [ERROR] db: connection lost
121 [INFO] auth: user alice logged in
130 [WARN] net: retrying in 5s
x [INFO] bad: timestamp
140 [ERROR] net: timeout after 3 retries
150 [INFO] db: reconnected";
    let logs: Vec<LogLine> = raw.lines().filter_map(parse).collect();
    println!("parsed {} of {} lines", logs.len(), raw.lines().count());
    let errors: Vec<String> = logs.iter().filter(|l| l.level == "ERROR").map(|l| format!("{}@{}", l.module, l.ts)).collect();
    println!("errors: {:?}", errors);
    let mut modules: Vec<&str> = logs.iter().map(|l| l.module.as_str()).collect();
    modules.sort();
    modules.dedup();
    for m in &modules {
        let entries: Vec<&LogLine> = logs.iter().filter(|l| l.module == *m).collect();
        let worst = entries
            .iter()
            .map(|l| match l.level.as_str() {
                "ERROR" => 3,
                "WARN" => 2,
                _ => 1,
            })
            .max()
            .unwrap_or(0);
        println!("{:<5} count={} worst={} span={}..{}", m, entries.len(), worst, entries.first().unwrap().ts, entries.last().unwrap().ts);
    }
    let numbers: Vec<u32> = logs
        .iter()
        .flat_map(|l| l.msg.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse::<u32>().unwrap()))
        .collect();
    println!("numbers in messages: {:?}", numbers);
    let gaps: Vec<u32> = logs.windows(2).map(|w| w[1].ts - w[0].ts).collect();
    println!("gaps: {:?}", gaps);
    let max_gap = gaps.iter().enumerate().max_by_key(|(_, g)| **g).map(|(i, g)| (logs[i].ts, *g));
    println!("max gap after ts {:?}", max_gap);
    let first_warn_after_error = logs
        .iter()
        .skip_while(|l| l.level != "ERROR")
        .find(|l| l.level == "WARN")
        .map(|l| l.msg.clone());
    println!("first warn after error: {:?}", first_warn_after_error);
    let summary: String = logs.iter().map(|l| l.level.chars().next().unwrap()).collect();
    println!("level summary: {}", summary);
    let words_total: usize = logs.iter().map(|l| l.msg.split_whitespace().count()).sum();
    println!("total words: {}", words_total);
    let longest = logs.iter().max_by_key(|l| l.msg.len()).unwrap();
    println!("longest message: {:?}", longest.msg);
}
