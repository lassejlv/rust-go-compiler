// std::process::Command: output capture, status codes, stdin piping, env, current_dir.
use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn main() {
    let out = Command::new("echo").arg("hello").arg("world").output().unwrap();
    println!("status {} stdout {:?} stderr {:?}", out.status.success(), String::from_utf8_lossy(&out.stdout), out.stderr);

    let out = Command::new("sh").args(["-c", "echo out; echo err 1>&2; exit 3"]).output().unwrap();
    println!("code {:?} success {} out {:?} err {:?}", out.status.code(), out.status.success(),
        String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap());

    let st = Command::new("sh").arg("-c").arg("exit 0").status().unwrap();
    println!("status ok {}", st.success());

    let out = Command::new("sh").arg("-c").arg("echo $RG_VAR-$HOME_X").env("RG_VAR", "value").env("HOME_X", "h").output().unwrap();
    print!("env: {}", String::from_utf8_lossy(&out.stdout));
    let out = Command::new("sh").arg("-c").arg("echo ${RG_GONE:-unset}").env_remove("RG_GONE").env("RG_GONE", "x").env_remove("RG_GONE").output().unwrap();
    print!("env_remove: {}", String::from_utf8_lossy(&out.stdout));
    let out = Command::new("pwd").current_dir("/").output().unwrap();
    print!("pwd: {}", String::from_utf8_lossy(&out.stdout));

    // pipe stdin through `sort`
    let mut child = Command::new("sort").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"pear\napple\nmango\nbanana\n").unwrap();
    }
    let mut s = String::new();
    child.stdout.take().unwrap().read_to_string(&mut s).unwrap();
    let st = child.wait().unwrap();
    println!("sorted {:?} {}", s.lines().collect::<Vec<_>>(), st.success());

    // wait_with_output
    let child = Command::new("sh").args(["-c", "printf 'a\\nb\\nc'; exit 7"]).stdout(Stdio::piped()).spawn().unwrap();
    let o = child.wait_with_output().unwrap();
    println!("wwo {:?} {:?}", o.status.code(), String::from_utf8_lossy(&o.stdout));

    // stdout null
    let st = Command::new("echo").arg("hidden").stdout(Stdio::null()).status().unwrap();
    println!("null ok {}", st.success());
    // spawning a missing program
    let e = Command::new("definitely-not-a-real-program-xyz").output();
    println!("missing: {:?}", e.map(|_| ()).unwrap_err().kind());
    // killed by signal -> code None
    let o = Command::new("sh").args(["-c", "kill -9 $$"]).output().unwrap();
    println!("signal code {:?} success {}", o.status.code(), o.status.success());
    let mut c = Command::new("sh");
    c.arg("-c").arg("echo $0 $1 $2").arg("zero").arg("one").arg("two");
    println!("program {:?} args {:?}", c.get_program(), c.get_args().collect::<Vec<_>>());
    print!("{}", String::from_utf8_lossy(&c.output().unwrap().stdout));
}
