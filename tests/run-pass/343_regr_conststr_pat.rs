mod record { pub const NAME: &str = "RECORD"; }
mod shape { pub const NAME: &str = "SHAPE"; }
#[derive(Debug, Clone, Copy)] struct Info { first: u8 }
fn classify(ext: Option<(&str, Info)>, code: u8) -> String {
    match ext {
        Some((record::NAME, info)) => format!("record {}", code - info.first),
        Some((shape::NAME, info)) => format!("shape {}", code - info.first),
        _ => "unknown".to_string(),
    }
}
fn main() {
    println!("{}", classify(Some(("RECORD", Info { first: 3 })), 5));
    println!("{}", classify(Some(("SHAPE", Info { first: 1 })), 5));
    println!("{}", classify(None, 5));
}
