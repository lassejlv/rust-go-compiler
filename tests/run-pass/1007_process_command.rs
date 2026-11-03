use std::io::Write;
use std::process::{Command, Stdio};

fn main() {
    let out = Command::new("echo").arg("hello").arg("from echo").output().expect("echo");
    println!("status ok {} stdout {:?}", out.status.success(), String::from_utf8_lossy(&out.stdout));

    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("cat");
    {
        let mut stdin = child.stdin.take().unwrap();
        for i in 0..3 {
            writeln!(stdin, "piped line {}", i).unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    print!("{}", String::from_utf8(out.stdout).unwrap());

    let st = Command::new("sh").args(["-c", "exit 3"]).status().unwrap();
    println!("exit code {:?} success {}", st.code(), st.success());

    let out = Command::new("sh")
        .arg("-c")
        .arg("echo $RG_TEST_VAR; echo err >&2")
        .env("RG_TEST_VAR", "env-passed")
        .output()
        .unwrap();
    println!("{:?} {:?}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));

    let out = Command::new("sh").arg("-c").arg("pwd").current_dir("/").output().unwrap();
    println!("pwd {:?}", String::from_utf8_lossy(&out.stdout).trim());

    let r = Command::new("definitely-not-a-real-binary-xyz").output();
    println!("missing binary err kind {:?}", r.map(|_| ()).map_err(|e| e.kind()));

    let out = Command::new("sh").arg("-c").arg("echo $RG_CLEARED").env_clear().env("RG_CLEARED", "only").output().unwrap();
    println!("{}", String::from_utf8_lossy(&out.stdout).trim());

    let echo = Command::new("printf").arg("c\\nb\\na\\n").stdout(Stdio::piped()).spawn().unwrap();
    let sort = Command::new("sort").stdin(Stdio::from(echo.stdout.unwrap())).output().unwrap();
    println!("sorted {:?}", String::from_utf8_lossy(&sort.stdout));
    let mut c = Command::new("true");
    let s = c.spawn().unwrap().wait().unwrap();
    println!("true -> {}", s);
    let s = Command::new("false").status().unwrap();
    println!("false -> {:?}", s.code());
    let cmd = Command::new("ls");
    println!("program {:?}", cmd.get_program());
}
