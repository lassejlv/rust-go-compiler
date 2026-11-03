use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> io::Result<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let rel = p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
        let ft = e.file_type()?;
        if ft.is_dir() {
            out.push(format!("{}/", rel));
            walk(&p, base, out)?;
        } else {
            out.push(format!("{} ({} bytes)", rel, e.metadata()?.len()));
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let root: PathBuf = std::env::temp_dir().join(format!("rg1005_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("a/b/c"))?;
    fs::create_dir(root.join("z"))?;
    fs::write(root.join("a/one.txt"), "1")?;
    fs::write(root.join("a/b/two.txt"), "22")?;
    fs::write(root.join("a/b/c/three.rs"), "333")?;
    fs::write(root.join("top.md"), "# hi\n")?;
    println!("create_dir existing err: {:?}", fs::create_dir(root.join("z")).map_err(|e| e.kind()));
    let mut out = vec![];
    walk(&root, &root, &mut out)?;
    for l in &out { println!("{}", l); }
    let md = fs::metadata(root.join("a"))?;
    println!("a is_dir {} is_file {}", md.is_dir(), md.is_file());
    let p = root.join("a/b/c/three.rs");
    println!("ext {:?} stem {:?} name {:?}", p.extension(), p.file_stem(), p.file_name());
    println!("parent ends with c: {}", p.parent().unwrap().ends_with("b/c"));
    let comps: Vec<_> = p.strip_prefix(&root).unwrap().components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    println!("{:?}", comps);
    let mut pb = PathBuf::from("x/y");
    pb.push("z.tar.gz");
    pb.set_extension("zip");
    println!("{}", pb.display());
    pb.pop();
    println!("{} abs={}", pb.display(), pb.is_absolute());
    println!("with_file_name {}", Path::new("d/e.txt").with_file_name("f.txt").display());
    fs::remove_dir(root.join("z"))?;
    println!("remove nonempty err: {}", fs::remove_dir(root.join("a")).is_err());
    fs::remove_dir_all(&root)?;
    println!("root exists after: {}", root.exists());
    println!("try_exists {:?}", root.try_exists().ok());
    Ok(())
}
