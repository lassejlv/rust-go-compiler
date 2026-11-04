//~ ERROR E0170
enum Dir { North, South }
fn name(d: Dir) -> &'static str {
    match d {
        North => "north",
        _ => "south",
    }
}
fn main() { println!("{}", name(Dir::South)); }
