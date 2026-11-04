// Unix fs extras: permissions, symlinks, read_link, canonicalize, DirBuilder recursive,
// Path components/ancestors/strip_prefix, OsStr bytes, ExitStatusExt::signal.
use std::ffi::OsStr;
use std::fs::{self, DirBuilder, File};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

fn main() -> std::io::Result<()> {
    let root = std::env::temp_dir().join(format!("rg_1292_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    DirBuilder::new().recursive(true).create(root.join("a/b/c"))?;
    let f = root.join("a/b/c/file.sh");
    {
        let mut fh = File::create(&f)?;
        fh.write_all(b"#!/bin/sh\necho from-script $1\nexit 5\n")?;
    }
    let mut perms = fs::metadata(&f)?.permissions();
    println!("readonly {}", perms.readonly());
    perms.set_mode(0o750);
    fs::set_permissions(&f, perms)?;
    println!("mode {:o}", fs::metadata(&f)?.permissions().mode() & 0o777);
    let out = Command::new(&f).arg("arg1").output()?;
    println!("{:?} {:?}", String::from_utf8_lossy(&out.stdout), out.status.code());
    println!("signal {:?}", out.status.signal());

    let link = root.join("link");
    symlink("a/b/c/file.sh", &link)?;
    println!("read_link {:?}", fs::read_link(&link)?);
    println!("is_symlink {} meta follows {}", fs::symlink_metadata(&link)?.file_type().is_symlink(), fs::metadata(&link)?.is_file());
    let canon = fs::canonicalize(&link)?;
    println!("canon ends {}", canon.ends_with("a/b/c/file.sh"));
    println!("strip {:?}", canon.strip_prefix(fs::canonicalize(&root)?).unwrap());

    let p = Path::new("/usr/./local/../lib/libfoo.so.1");
    let comps: Vec<String> = p
        .components()
        .map(|c| match c {
            Component::RootDir => "/".to_string(),
            Component::CurDir => ".".to_string(),
            Component::ParentDir => "..".to_string(),
            Component::Normal(s) => s.to_string_lossy().into_owned(),
            Component::Prefix(_) => "prefix".to_string(),
        })
        .collect();
    println!("{:?}", comps);
    let anc: Vec<&Path> = Path::new("x/y/z").ancestors().collect();
    println!("{:?}", anc);
    println!("{:?} {:?} {:?}", p.extension(), p.file_stem(), p.parent());
    let mut pb = PathBuf::from("/tmp/base");
    pb.push("sub");
    pb.set_extension("tar.gz");
    println!("{} {:?}", pb.display(), pb.with_file_name("other.txt"));
    pb.pop();
    println!("{:?} abs {} rel {}", pb, pb.is_absolute(), Path::new("rel/p").is_relative());
    println!("{:?}", Path::new("a").join("/abs"));
    let os = OsStr::from_bytes(b"bytes\xffname");
    println!("{:?} {:?} {}", os, os.to_string_lossy(), os.as_bytes().len());
    println!("starts_with {} {}", Path::new("/a/b/c").starts_with("/a/b"), Path::new("/a/bc").starts_with("/a/b"));

    let mut names = vec![];
    fn walk(dir: &Path, base: &Path, names: &mut Vec<String>) -> std::io::Result<()> {
        for e in fs::read_dir(dir)? {
            let e = e?;
            let ft = e.file_type()?;
            let rel = e.path().strip_prefix(base).unwrap().to_string_lossy().into_owned();
            names.push(format!("{}{}", rel, if ft.is_dir() { "/" } else if ft.is_symlink() { "@" } else { "" }));
            if ft.is_dir() {
                walk(&e.path(), base, names)?;
            }
        }
        Ok(())
    }
    walk(&root, &root, &mut names)?;
    names.sort();
    println!("{:?}", names);
    let st = Command::new("sh").args(["-c", "kill -TERM $$"]).status()?;
    println!("term signal {:?} code {:?}", st.signal(), st.code());
    fs::remove_dir_all(&root)?;
    println!("removed {}", !root.exists());
    Ok(())
}
