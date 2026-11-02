use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();
    println!("cargo:rustc-cfg=built_for_{}", if target.contains("linux") { "linux" } else { "other" });
    if env::var("CARGO_FEATURE_SHINY").is_ok() {
        println!("cargo::rustc-cfg=shiny_enabled");
    }
    let n: u32 = (1..=5).sum();
    println!("cargo:rustc-env=BUILD_SUM={}", n);
    println!("cargo:rerun-if-changed=build.rs");
}
