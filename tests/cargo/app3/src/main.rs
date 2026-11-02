fn main() {
    println!("{} v{} ({})", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_DESCRIPTION"));
    println!("major={} minor={}", env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"));
    #[cfg(built_for_linux)]
    println!("built for linux");
    #[cfg(shiny_enabled)]
    println!("shiny!");
    println!("sum from build script = {}", env!("BUILD_SUM"));
    println!("missing = {:?}", option_env!("RUSTGO_NOT_SET_ANYWHERE"));
}
