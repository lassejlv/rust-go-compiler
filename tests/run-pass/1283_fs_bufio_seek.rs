// std::fs in a private temp dir: write/read, BufWriter/BufReader, lines, seek, metadata, rename.
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

fn tmpdir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("rg_1283_{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn main() -> io::Result<()> {
    let dir = tmpdir();
    let p = dir.join("data.txt");
    {
        let f = File::create(&p)?;
        let mut w = BufWriter::new(f);
        for i in 0..5 {
            writeln!(w, "line {} {}", i, "x".repeat(i))?;
        }
        w.write_all(b"no newline at end")?;
        w.flush()?;
    }
    let meta = fs::metadata(&p)?;
    println!("is_file {} len {}", meta.is_file(), meta.len());
    let s = fs::read_to_string(&p)?;
    println!("chars {} lines {}", s.len(), s.lines().count());
    let r = BufReader::new(File::open(&p)?);
    for (n, line) in r.lines().enumerate() {
        let line = line?;
        println!("{}: {:?}", n, line);
    }
    let mut f = File::open(&p)?;
    f.seek(SeekFrom::Start(5))?;
    let mut buf = [0u8; 6];
    f.read_exact(&mut buf)?;
    println!("at 5: {:?}", std::str::from_utf8(&buf).unwrap());
    let pos = f.seek(SeekFrom::End(-6))?;
    let mut tail = String::new();
    f.read_to_string(&mut tail)?;
    println!("pos {} tail {:?}", pos, tail);
    f.seek(SeekFrom::Current(-3))?;
    println!("stream_position {}", f.stream_position()?);
    f.rewind()?;
    let mut first = String::new();
    BufReader::new(&mut f).read_line(&mut first)?;
    println!("first {:?}", first);

    // append and overwrite with OpenOptions
    {
        let mut a = OpenOptions::new().append(true).open(&p)?;
        a.write_all(b"\nappended")?;
    }
    {
        let mut o = OpenOptions::new().write(true).open(&p)?;
        o.write_all(b"LINE")?;
    }
    let all = fs::read(&p)?;
    println!("starts {:?} ends {:?}", std::str::from_utf8(&all[..10]).unwrap(), std::str::from_utf8(&all[all.len() - 8..]).unwrap());

    // read_until / split on a custom delimiter
    fs::write(dir.join("csv"), "a,bb,,ccc")?;
    let rd = BufReader::new(File::open(dir.join("csv"))?);
    let parts: Vec<String> = rd.split(b',').map(|r| String::from_utf8(r.unwrap()).unwrap()).collect();
    println!("{:?}", parts);

    // create_new fails when exists
    let e = OpenOptions::new().write(true).create_new(true).open(&p);
    println!("create_new exists err kind {:?}", e.map(|_| ()).unwrap_err().kind());
    let e = File::open(dir.join("missing"));
    println!("missing kind {:?}", e.map(|_| ()).unwrap_err().kind());

    // directories, rename, copy, read_dir sorted
    fs::create_dir(dir.join("sub"))?;
    fs::rename(dir.join("csv"), dir.join("sub").join("moved.csv"))?;
    let n = fs::copy(&p, dir.join("sub").join("copy.txt"))?;
    println!("copied {} bytes", n);
    let mut names: Vec<String> = fs::read_dir(dir.join("sub"))?
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    println!("{:?}", names);
    println!("exists {} {}", dir.join("csv").exists(), dir.join("sub/moved.csv").exists());
    fs::remove_file(dir.join("sub").join("copy.txt"))?;
    println!("after remove {}", fs::read_dir(dir.join("sub"))?.count());
    let ft = fs::metadata(dir.join("sub"))?.file_type();
    println!("sub is_dir {}", ft.is_dir());
    let pb = dir.join("sub").join("moved.csv");
    println!("{:?} {:?} {:?}", pb.file_name(), pb.extension(), pb.file_stem());

    // set_len truncation
    let f = OpenOptions::new().write(true).open(&p)?;
    f.set_len(4)?;
    drop(f);
    println!("truncated {:?}", fs::read_to_string(&p)?);
    fs::remove_dir_all(&dir)?;
    println!("cleaned {}", !dir.exists());
    Ok(())
}
