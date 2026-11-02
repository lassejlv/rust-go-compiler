// {:?} escaping rules for str and char: quotes, backslashes, control chars, \u{..} for
// non-printable/unassigned, combining marks (grapheme extend) at start vs middle, zero-width,
// non-ASCII printable kept verbatim, and Debug of nested containers of strings/chars/bytes.
use std::collections::BTreeMap;

#[derive(Debug)]
struct Msg<'a> {
    text: &'a str,
    sep: char,
    raw: &'a [u8],
}

fn main() {
    let strs = [
        "plain",
        "with \"double\" quotes",
        "with 'single' quotes",
        "back\\slash",
        "nl\ncr\rtab\t",
        "nul\0bell\x07esc\x1b del\x7f",
        "unicode: é ñ 日本 🦀",
        "\u{301}combining at start",
        "mid\u{301}dle",
        "zero\u{200b}width\u{feff}bom",
        "nbsp\u{a0}here",
        "line\u{2028}sep",
        "private\u{e000}use",
        "unassigned\u{378}x",
        "\u{10ffff}",
        "soft\u{ad}hyphen",
        "rtl\u{202e}override",
        "",
    ];
    for s in strs {
        println!("{:?}", s);
    }

    let chars = ['"', '\'', '\\', '\n', '\u{0}', '\u{301}', '\u{200b}', 'é', '\u{ad}', '\u{85}', ' '];
    for c in chars {
        print!("{:?} ", c);
    }
    println!();

    let v = vec!["a\"b", "c\\d", "e\nf"];
    println!("{:?}", v);
    println!("{:#?}", v);
    let t = ('\t', "\t", b'\t');
    println!("{:?}", t);
    let m: BTreeMap<&str, Vec<char>> = [("k\"1", vec!['\'', '"']), ("k2", vec![])].into_iter().collect();
    println!("{:?}", m);
    let msg = Msg { text: "say \"hi\"\n", sep: '\'', raw: b"\x00\xffA" };
    println!("{:?}", msg);
    println!("{:#?}", msg);

    let owned = String::from("own\\ed");
    let boxed: Box<str> = "box\"ed".into();
    let cow: std::borrow::Cow<str> = std::borrow::Cow::Borrowed("c'ow");
    println!("{:?} {:?} {:?}", owned, boxed, cow);
    println!("{:?}", Some("x\ty"));
    println!("{:?}", Ok::<&str, ()>("\u{1F600}"));
    println!("{:?}", ["\u{7}"; 2]);

    // width/padding with Debug strings applies to the escaped form
    println!("[{:>12?}] [{:<10?}] [{:^9?}]", "a\nb", "q\"", 'x');
    // Display vs Debug comparison
    for s in ["tab\there", "quote\"", "uni\u{e9}"] {
        println!("display=<{}> debug=<{:?}> len={}", s, s, format!("{:?}", s).len());
    }
    let bytes = "ab\n".as_bytes();
    println!("{:?} {:x?}", bytes, bytes);
    let path = std::path::Path::new("/tmp/with space/\"q\"");
    println!("{:?}", path);
    println!("{:?}", path.display().to_string());
    println!("{:?}", std::ffi::OsStr::new("os\nstr"));
    let escaped: String = "a\"b\\c\nd".chars().flat_map(|c| c.escape_debug()).collect();
    println!("{}", escaped);

    // JSON-style escaping implemented by hand, compared against Debug
    fn json_escape(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }
    for s in ["simple", "q\"uote", "ctl\x01\x1f", "nl\n", "uni é"] {
        let j = json_escape(s);
        let d = format!("{:?}", s);
        println!("{} {} same={}", j, d, j == d);
    }
}
