use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("argc {}", args.len());
    println!("argv0 nonempty {}", !args[0].is_empty());
    println!("args().skip(1) {:?}", env::args().skip(1).collect::<Vec<_>>());
    env::set_var("RG1008_A", "alpha");
    env::set_var("RG1008_B", "beta=gamma");
    println!("{:?}", env::var("RG1008_A"));
    println!("{:?}", env::var("RG1008_MISSING"));
    let mut mine: Vec<(String, String)> = env::vars().filter(|(k, _)| k.starts_with("RG1008_")).collect();
    mine.sort();
    println!("{:?}", mine);
    env::remove_var("RG1008_A");
    println!("after remove {:?}", env::var("RG1008_A").is_err());
    println!("var_os {:?}", env::var_os("RG1008_B"));
    let paths = env::join_paths(["/usr/bin", "/bin"]).unwrap();
    println!("{:?}", paths);
    let split: Vec<_> = env::split_paths("/a:/b/c::/d").map(|p| p.display().to_string()).collect();
    println!("{:?}", split);
    println!("cwd absolute {}", env::current_dir().unwrap().is_absolute());
    println!("temp_dir absolute {}", env::temp_dir().is_absolute());
    println!("consts {} {}", env::consts::OS, env::consts::FAMILY);
    match env::var("RG1008_B") {
        Ok(v) if v.contains('=') => {
            println!("exiting with 17");
            process::exit(17);
        }
        _ => println!("unreachable"),
    }
}
