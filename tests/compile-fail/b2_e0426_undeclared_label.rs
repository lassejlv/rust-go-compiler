//~ ERROR E0426
fn main() {
    'outer: for i in 0..3 {
        for j in 0..3 { if i == j { break 'inner; } }
    }
}
