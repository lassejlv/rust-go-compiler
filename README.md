# rustgo — a Rust compiler written in Go

`rustgo` compiles Rust source code to native executables. It is a from-scratch
implementation in Go whose pipeline mirrors rustc's crate layout, using GNU C as
its backend IR (where rustc uses LLVM IR):

| rustc crate                         | rustgo package          | what it does |
|-------------------------------------|-------------------------|--------------|
| `rustc_lexer`                       | `compiler/lexer`        | tokens, spans, source map |
| `rustc_parse` / `rustc_ast`         | `compiler/parser`, `compiler/ast` | full Rust 2021 syntax → AST, AST printer, visitor |
| `rustc_expand` / `rustc_builtin_macros` | `compiler/expand`   | `macro_rules!`, `println!`/`format!`/`vec!`/`assert!`…, `#[derive]`, `#[cfg]` |
| `rustc_resolve`                     | `compiler/resolve`      | module tree, definitions, `use` imports, path resolution |
| `rustc_middle`                      | `compiler/middle`       | interned types, definitions, generics, modules |
| `rustc_hir_analysis` / `rustc_hir_typeck` / `rustc_trait_selection` | `compiler/typeck` | item collection, type inference, coercions, method probing (autoderef/autoref), trait selection, associated-type normalization, closures & capture analysis, pattern typing (match ergonomics) |
| `rustc_mir_build` / `rustc_borrowck` | `compiler/borrowck` | MIR-like CFG of every body, move/initialization checking, mutability, loans with non-lexical lifetimes, two-phase borrows, universal regions of signatures |
| `rustc_monomorphize` / `rustc_codegen_llvm` | `compiler/codegen` | monomorphization, drop glue & drop flags, vtables, pattern-match lowering, panics/unwinding, C emission |
| `library/std` (+ `core`, `alloc`)   | `compiler/stdlib/lib`   | the standard library, written in Rust and compiled by rustgo |
| `rustc_driver`                      | `compiler/driver`, `cmd/rustgo` | pipeline + CLI |

## Usage

```bash
go build -o rustgo ./cmd/rustgo
./rustgo hello.rs -o hello && ./hello
./rustgo run hello.rs
./rustgo build                    # in a cargo project: builds the whole crate graph
./rustgo build --release -o app   # gcc -O2, wrapping integer arithmetic (like cargo --release)
./rustgo test lib.rs [filter]     # rustc --test: build the #[test] harness and run it
./rustgo test [filter] [-- args]  # in a cargo project: like `cargo test`
```

Options: `-o <out>`, `--release`, `--emit-c <file>` (dump generated C),
`--keep-c`, `--std <dir>` (use a std from a directory), `--cc <compiler>`
(default `gcc`), `-O <level>`, `-v` (phase timings).

`rustgo build` reads the crate graph with `cargo metadata --offline` (crates
must already be in the local cargo registry or be path dependencies), runs
build scripts, compiles proc-macro crates into macro servers, and compiles
everything into one native executable. Large programs are split into several
C translation units compiled in parallel.

`rustgo test` builds test harnesses like `rustc --test` (`--test` builds one
without running it): `#[test]` functions (`#[ignore]`, `#[should_panic]`,
`Result`-returning tests) are run by a libtest-compatible runner in std
(`std::rustgo_test`) with the same output, output capture and options
(filters, `--exact`, `--skip`, `--ignored`, `--include-ignored`,
`--nocapture`, `--list`). In a cargo project it builds the library and
binaries with `cfg(test)` and dev-dependencies plus every integration test
in `tests/`, and runs them from the package root (`--lib`, `--bins`,
`--bin N`, `--test N`, `--doc`, `--no-run`, `--no-fail-fast`), then the
library's doc-tests (code blocks in doc comments, with hidden `# ` lines,
`should_panic`, `no_run`, `compile_fail`, `ignore`); all doc-tests of a crate
are merged into one program (falling back to one program per test when the
merged one does not compile).

## What works

* Rust 2015/2018/2021/2024 syntax, `macro_rules!` (with hygiene for local
  variables), procedural macros (derive, attribute and bang macros, served by
  separately compiled macro servers), `#[cfg]`/`cfg_attr`, `include!`.
* Traits, generics, associated types and consts, GATs, `impl Trait` (argument
  and return position, also in traits), `dyn Trait` with supertrait upcasting,
  const generics with a compile-time const-fn interpreter, closures, iterators,
  pattern matching with match ergonomics, `?`, `async`/`.await` and
  `AsyncFn*` (coroutine-based), threads, atomics, `catch_unwind`, panic hooks,
  `#[track_caller]`, inline assembly (`asm!`, x86-64), FFI.
* A large standard library: collections, strings and formatting, io, fs,
  path, process, net, os::unix, sync/mpsc, thread-locals, arch::x86_64
  intrinsics (emulated), ...
* Real crates from crates.io build and produce the same output as rustc/cargo,
  e.g. regex (+ regex-syntax, aho-corasick, memchr), serde (hand-written
  serializers), nom, itertools, hashbrown, anyhow, once_cell, bitflags, log,
  uuid, itoa, ryu, bumpalo, crossbeam-utils, getrandom, rustix (libc and raw
  syscall backends), polling, num_cpus, ttf-parser, zlib-rs, hybrid-array,
  typenum, x11rb-protocol and many smaller ones.
* Invalid programs are rejected with rustc's error codes
  (`file:line:col: error[E0XXX]: ...`): besides resolution and type errors,
  match exhaustiveness and refutable bindings (E0004/E0005, Maranget's
  usefulness algorithm with integer-range and slice-length splitting),
  privacy of items, fields and methods (E0603/E0616/E0451/E0624), unsafety
  (E0133), trait impl conformance (E0046/E0407/E0050/E0053), coherence and
  orphan rules (E0119/E0117), duplicate definitions (E0428/E0592), casts,
  dyn compatibility, constant evaluation errors (E0080) and more. These
  checks give up silently on anything they do not model, and items of the
  embedded std (whose visibilities and signatures do not mirror the real
  std's in every detail) are exempt from the privacy and impl checks.

* A borrow checker (`compiler/borrowck`): use of moved or uninitialized
  values (E0382/E0381), moves out of borrows (E0507/E0508/E0509),
  mutability (E0596/E0594/E0384), conflicting borrows with non-lexical
  lifetimes and two-phase borrows (E0499/E0502/E0503/E0505/E0506),
  dangling references (E0597/E0716/E0515), closures outliving their
  captures (E0373), and signature lifetimes (E0621, "lifetime may not live
  long enough"). Types carry no lifetimes after type checking, so regions
  are modeled by loan sets attached to values (by reference depth) and
  propagated through calls by the callees' declared signatures; whenever
  the model is unsure it drops loans, so valid programs are never rejected
  (some invalid ones are accepted). It checks the crates being developed
  (the input file, cargo path dependencies); `RUSTGO_BORROWCK=all` also
  checks std and registry crates, `RUSTGO_BORROWCK=0` turns it off.
  `--check` stops after analysis (like `cargo check`).

Not implemented: targets other than x86_64-linux-gnu, incremental
compilation.

## Design notes

* **Generics are type-checked per instantiation** (template style): a generic
  function body is checked with its type parameters substituted by concrete
  types. Trait method calls and associated types therefore always resolve
  against concrete types, which makes trait selection and normalization simple
  and complete for the programs that rustc accepts.
* **Trait selection** supports user impls, blanket impls with where clauses,
  built-in impls (`Copy`/`Clone` for structural types, `Fn*` for closures, fn
  items and fn pointers, `Sized`, auto traits), trait objects and supertraits.
  Selection drives inference (e.g. `collect::<Vec<_>>()`).
* **Closures** get their signature from expected `Fn*` bounds; captures and
  the closure kind are computed by a use-analysis pass (by-ref, by-mut-ref or
  by-move, like rustc). From Rust 2021 on, closures capture disjoint places
  (`t.1`, `(*r).a.b`) with rustc's precision rules; moves out of fields of
  locals get per-field drop flags.
* **Formatting**: `format_args!` lowers to a call of `fmt::__format` with
  static pieces/specs and an array of `&dyn Trait` arguments (`print!` calls
  `io::__print` directly), so each use is a few lines of C.
* **Drops** are elaborated with runtime drop flags, including partial moves
  out of struct fields, temporaries with Rust's temporary-lifetime rules, and
  unwinding on panic (a cleanup stack runs pending destructors before the
  process exits with code 101).
* **Type layouts** follow rustc's repr(Rust) rules: fields are reordered by
  alignment, enum tags are as small as possible and niches (null pointers,
  invalid bools/chars, unused tag values) store other variants, so
  `size_of` matches rustc (`Option<&T>` = 8, `Option<u32>` = 8,
  `Result<u64, String>` = 24) and `repr(C)`/`repr(u8)` types are
  FFI-compatible.
* **Integer overflow** panics like a debug build of rustc (wraps with
  `--release`); bounds checks, division by zero etc. produce rustc's panic
  messages at the same source locations (including `#[track_caller]`).
* **Trait evaluation** is cached by canonical query (inference variables
  renumbered), and generic bodies can also be checked generically to pick the
  trait a method call meant when a substituted type implements several.
* The **standard library** is regular Rust in `compiler/stdlib/lib`, built on a
  small set of compiler intrinsics (`extern "rust-intrinsic"`), and embedded
  into the binary.

## Tests

`tests/run-pass` holds Rust programs with expected output generated by the real
`rustc`. Run them with:

```bash
tests/run.sh          # all tests
tests/run.sh -q       # only failures + summary
tests/run.sh 042      # tests matching a pattern
```

`tests/compile-fail` holds programs rustc rejects (with the expected error
code); `tests/compile-fail/run.sh -q` checks that rustgo rejects them too.

`tests/cargo/run.sh` builds the Cargo projects under `tests/cargo` (path
dependencies, proc macros, build scripts, `rustgo test`).

Unit tests: `go test ./...`.
