#[cfg(feature = "nope")]
fn main() -> std::io::Result<()> {
    Ok(())
}
#[cfg(not(feature = "nope"))]
fn main() { println!("dummy"); }
