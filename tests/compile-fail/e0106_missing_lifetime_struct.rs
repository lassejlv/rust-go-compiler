//~ ERROR E0106
struct View {
    data: &[u8],
}

fn main() {
    let v = View { data: &[1, 2] };
    println!("{}", v.data.len());
}
