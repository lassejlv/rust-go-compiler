use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::os::unix::fs::PermissionsExt;

fn main() -> io::Result<()> {
    let dir = std::env::temp_dir().join(format!("rg1031_{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let p = dir.join("crlf.txt");
    fs::write(&p, b"first\r\nsecond\n\nfourth no newline")?;
    let mut r = BufReader::new(File::open(&p)?);
    let mut line = String::new();
    loop {
        line.clear();
        let n = r.read_line(&mut line)?;
        if n == 0 { break; }
        println!("read_line n={} {:?}", n, line);
    }
    let lines: Vec<String> = BufReader::new(File::open(&p)?).lines().collect::<Result<_, _>>()?;
    println!("lines {:?}", lines);
    let mut r = BufReader::with_capacity(4, File::open(&p)?);
    let buf = r.fill_buf()?.to_vec();
    println!("fill_buf {:?} cap {}", String::from_utf8_lossy(&buf), r.capacity());
    r.consume(2);
    let mut rest = String::new();
    r.read_to_string(&mut rest)?;
    println!("rest len {}", rest.len());

    let big = dir.join("big.txt");
    {
        let mut w = BufWriter::with_capacity(16, File::create(&big)?);
        for i in 0..1000 { writeln!(w, "{:04}", i)?; }
        println!("buffer len before flush <= 16: {}", w.buffer().len() <= 16);
        let f = w.into_inner().map_err(|e| e.into_error())?;
        f.sync_all()?;
    }
    let md = fs::metadata(&big)?;
    println!("big len {}", md.len());
    let sum: u32 = BufReader::new(File::open(&big)?).lines().map(|l| l.unwrap().parse::<u32>().unwrap()).sum();
    println!("sum {}", sum);
    {
        let mut f = OpenOptions::new().write(true).truncate(true).open(&big)?;
        f.write_all(b"short")?;
    }
    println!("after truncate {:?}", fs::read_to_string(&big)?);

    let mut perms = fs::metadata(&p)?.permissions();
    perms.set_mode(0o640);
    fs::set_permissions(&p, perms)?;
    println!("mode {:o}", fs::metadata(&p)?.permissions().mode() & 0o777);
    let mut ro = fs::metadata(&p)?.permissions();
    ro.set_readonly(true);
    fs::set_permissions(&p, ro)?;
    println!("readonly {}", fs::metadata(&p)?.permissions().readonly());
    let link = dir.join("link");
    std::os::unix::fs::symlink(&p, &link)?;
    println!("is_symlink {} target_is_crlf {}", fs::symlink_metadata(&link)?.file_type().is_symlink(),
        fs::read_link(&link)?.file_name().unwrap() == "crlf.txt");
    println!("via link len {}", fs::read(&link)?.len());
    let canon = fs::canonicalize(&link)?;
    println!("canonical ends with crlf.txt {}", canon.ends_with("crlf.txt"));
    fs::hard_link(&big, dir.join("hard"))?;
    println!("hard link content {:?}", fs::read_to_string(dir.join("hard"))?);
    let mut names: Vec<String> = fs::read_dir(&dir)?.map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    println!("{:?}", names);
    fs::remove_dir_all(&dir)?;
    println!("cleaned {}", !dir.exists());
    Ok(())
}
