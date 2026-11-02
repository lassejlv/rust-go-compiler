use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// Finds the type name and counts named fields of a struct.
fn struct_info(input: TokenStream) -> (String, usize, Vec<String>) {
    let mut name = String::new();
    let mut fields = Vec::new();
    let mut iter = input.into_iter().peekable();
    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(id) if id.to_string() == "struct" || id.to_string() == "enum" => {
                if let Some(TokenTree::Ident(n)) = iter.next() {
                    name = n.to_string();
                }
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace && !name.is_empty() => {
                let mut prev: Option<String> = None;
                for t in g.stream() {
                    match &t {
                        TokenTree::Punct(p) if p.as_char() == ':' => {
                            if let Some(f) = prev.take() {
                                fields.push(f);
                            }
                        }
                        TokenTree::Ident(i) => prev = Some(i.to_string()),
                        _ => prev = None,
                    }
                }
            }
            _ => {}
        }
    }
    let n = fields.len();
    (name, n, fields)
}

#[proc_macro_derive(Describe, attributes(describe))]
pub fn derive_describe(input: TokenStream) -> TokenStream {
    let (name, n, fields) = struct_info(input);
    let src = format!(
        "impl Describe for {name} {{ fn describe() -> String {{ String::from(\"{name} with {n} fields: {}\") }} }}",
        fields.join(", ")
    );
    src.parse().unwrap()
}

#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    let label = attr.to_string();
    let mut out = Vec::new();
    let mut fname = String::new();
    let mut seen_fn = false;
    for tt in item {
        match &tt {
            TokenTree::Ident(id) if id.to_string() == "fn" => seen_fn = true,
            TokenTree::Ident(id) if seen_fn && fname.is_empty() => fname = id.to_string(),
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                let body = g.stream();
                let wrapped = format!(
                    "{{ println!(\"enter {} [{}]\"); let __r = (|| {}  )(); println!(\"exit {}\"); __r }}",
                    fname, label, Group::new(Delimiter::Brace, body), fname
                );
                out.push(TokenTree::Group(Group::new(Delimiter::Brace, wrapped.parse::<TokenStream>().unwrap().into_iter().next().map(|t| match t { TokenTree::Group(g) => g.stream(), other => TokenStream::from(other) }).unwrap())));
                continue;
            }
            _ => {}
        }
        out.push(tt);
    }
    out.into_iter().collect()
}

#[proc_macro]
pub fn make_answer(input: TokenStream) -> TokenStream {
    let n: u32 = input.to_string().trim().parse().unwrap_or(42);
    let mut ts = TokenStream::new();
    ts.extend(vec![
        TokenTree::Ident(Ident::new("fn", Span::call_site())),
        TokenTree::Ident(Ident::new("answer", Span::call_site())),
        TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::new())),
        TokenTree::Punct(Punct::new('-', Spacing::Joint)),
        TokenTree::Punct(Punct::new('>', Spacing::Alone)),
        TokenTree::Ident(Ident::new("u32", Span::call_site())),
        TokenTree::Group(Group::new(Delimiter::Brace, TokenStream::from(TokenTree::Literal(Literal::u32_unsuffixed(n))))),
    ]);
    ts
}
