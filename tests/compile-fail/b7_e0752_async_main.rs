//~ ERROR E0752
// `main` cannot be async without a runtime attribute.
async fn main() {
    println!("hi");
}
