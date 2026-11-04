// Child pipelines (stdout of one into stdin of another), Stdio::from(File) redirects,
// try_wait/kill, large interleaved stdout+stderr capture, env vars and current_dir.
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

fn main() {
    let dir = env::temp_dir().join(format!("rg_1303_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // pipeline: printf | tr | sort -r
    let mut p1 = Command::new("printf").arg("delta\\nalpha\\ncharlie\\nbravo\\n").stdout(Stdio::piped()).spawn().unwrap();
    let mut p2 = Command::new("tr").args(["a-z", "A-Z"]).stdin(Stdio::from(p1.stdout.take().unwrap())).stdout(Stdio::piped()).spawn().unwrap();
    let p3 = Command::new("sort").arg("-r").stdin(Stdio::from(p2.stdout.take().unwrap())).output().unwrap();
    p1.wait().unwrap();
    p2.wait().unwrap();
    println!("{:?}", String::from_utf8(p3.stdout).unwrap().lines().collect::<Vec<_>>());

    // redirect child stdout into a file, then read it back
    let out_path = dir.join("out.txt");
    let f = File::create(&out_path).unwrap();
    let st = Command::new("sh").args(["-c", "echo line1; echo line2"]).stdout(Stdio::from(f)).status().unwrap();
    println!("status {} file {:?}", st.success(), fs::read_to_string(&out_path).unwrap());
    // stdin from a file
    let o = Command::new("wc").arg("-l").stdin(File::open(&out_path).unwrap()).output().unwrap();
    println!("wc -l: {}", String::from_utf8_lossy(&o.stdout).trim());

    // large interleaved output on both pipes (must not deadlock)
    let o = Command::new("sh")
        .args(["-c", "i=0; while [ $i -lt 3000 ]; do echo out$i; echo err$i 1>&2; i=$((i+1)); done"])
        .output()
        .unwrap();
    println!("out lines {} err lines {} last out {:?}", o.stdout.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count(),
        o.stderr.split(|&b| b == b'\n').filter(|l| !l.is_empty()).count(),
        String::from_utf8_lossy(&o.stdout).lines().last());

    // streaming read of a child's stdout line by line
    let mut child = Command::new("sh").args(["-c", "for w in one two three; do echo $w; done"]).stdout(Stdio::piped()).spawn().unwrap();
    let rd = BufReader::new(child.stdout.take().unwrap());
    let lens: Vec<usize> = rd.lines().map(|l| l.unwrap().len()).collect();
    println!("lens {:?} exit {:?}", lens, child.wait().unwrap().code());

    // try_wait + kill on a long-running child
    let mut sleeper = Command::new("sleep").arg("30").spawn().unwrap();
    println!("running: {}", sleeper.try_wait().unwrap().is_none());
    println!("has id: {}", sleeper.id() > 0);
    sleeper.kill().unwrap();
    let st = sleeper.wait().unwrap();
    println!("killed: success={} code={:?}", st.success(), st.code());

    // feed stdin while reading stdout from a thread
    let mut cat = Command::new("cat").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut cin = cat.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        for i in 0..2000 {
            writeln!(cin, "row {}", i).unwrap();
        }
    });
    let mut all = String::new();
    cat.stdout.take().unwrap().read_to_string(&mut all).unwrap();
    writer.join().unwrap();
    cat.wait().unwrap();
    println!("cat rows {} bytes {}", all.lines().count(), all.len());

    // env vars and current_dir
    env::set_var("RG_1303_VAR", "set-value");
    println!("{:?} {:?}", env::var("RG_1303_VAR"), env::var("RG_1303_MISSING").is_err());
    let o = Command::new("sh").args(["-c", "echo $RG_1303_VAR"]).output().unwrap();
    print!("child sees {}", String::from_utf8_lossy(&o.stdout));
    env::remove_var("RG_1303_VAR");
    let o = Command::new("sh").args(["-c", "echo ${RG_1303_VAR:-gone}"]).env_clear().env("PATH", env::var("PATH").unwrap()).output().unwrap();
    print!("after remove {}", String::from_utf8_lossy(&o.stdout));
    let found = env::vars().any(|(k, _)| k == "PATH");
    println!("PATH present {}", found);
    let old = env::current_dir().unwrap();
    env::set_current_dir(&dir).unwrap();
    fs::write("relative.txt", "rel").unwrap();
    let o = Command::new("cat").arg("relative.txt").output().unwrap();
    println!("cwd child read {:?}", String::from_utf8_lossy(&o.stdout));
    env::set_current_dir(&old).unwrap();
    fs::remove_dir_all(&dir).unwrap();
    println!("cleaned {}", !dir.exists());
}
