use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;

fn main() {
    // println! is line-buffered: flushed before the child writes to the shared stdout
    println!("parent before child");
    let st = Command::new("echo").arg("child inherits stdout").status().unwrap();
    println!("child status {:?}", st.code());
    std::io::stdout().flush().unwrap();

    // big stdin written from another thread while we read stdout (avoid pipe deadlock)
    let mut child = Command::new("wc")
        .arg("-l")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let writer = thread::spawn(move || {
        for i in 0..20000 {
            writeln!(stdin, "line {}", i).unwrap();
        }
    });
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    writer.join().unwrap();
    let st = child.wait().unwrap();
    println!("wc says {} ({})", out.trim(), st.success());

    let out = Command::new("sh").arg("-c").arg("echo to-null; echo kept >&2").stdout(Stdio::null()).stderr(Stdio::piped()).output().unwrap();
    println!("stdout empty {} stderr {:?}", out.stdout.is_empty(), String::from_utf8_lossy(&out.stderr));

    let mut child = Command::new("sleep").arg("5").spawn().unwrap();
    println!("try_wait running {:?}", child.try_wait().unwrap().is_none());
    child.kill().unwrap();
    let st = child.wait().unwrap();
    println!("killed success={} code={:?}", st.success(), st.code());
    use std::os::unix::process::ExitStatusExt;
    println!("signal {:?}", st.signal());

    let out = Command::new("sh").arg("-c").arg("read a; read b; echo \"$b-$a\"").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().and_then(|mut c| {
        c.stdin.as_mut().unwrap().write_all(b"first\nsecond\n")?;
        c.wait_with_output()
    }).unwrap();
    println!("{}", String::from_utf8_lossy(&out.stdout).trim_end());
    let mut cmd = Command::new("printf");
    cmd.args(["%s|", "a b", "c"]);
    let args: Vec<_> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    println!("args {:?}", args);
    println!("{}", String::from_utf8(cmd.output().unwrap().stdout).unwrap());
    print!("no newline yet ");
    std::io::stdout().flush().unwrap();
    Command::new("echo").arg("<child>").status().unwrap();
    println!("done");
}
