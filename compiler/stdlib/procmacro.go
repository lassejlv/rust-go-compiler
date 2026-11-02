package stdlib

import "embed"

// ProcMacroFS holds the source of the `proc_macro` crate (root:
// proc_macro/lib.rs), linked into procedural-macro servers.
//
//go:embed proc_macro
var ProcMacroFS embed.FS
