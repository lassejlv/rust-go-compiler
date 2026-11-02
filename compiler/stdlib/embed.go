// Package stdlib embeds the Rust source of the standard library crate
// (`std`, also reachable as `core` and `alloc`) compiled before every user
// crate.
package stdlib

import (
	"embed"
	"fmt"
	"io/fs"
	"path"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/parser"
)

// FS holds the library sources under "lib/".
//
//go:embed lib
var FS embed.FS

// RootPath is the crate root of the standard library inside FS.
const RootPath = "lib/lib.rs"

// LoadCrate parses the standard library: the crate root and, recursively,
// every out-of-line `mod foo;` (loaded from `foo.rs` or `foo/mod.rs`
// relative to the declaring file, following rustc's rules). addFile
// registers a source file and returns its file index (usually
// lexer.AddFile); the display name passed is the embed path.
func LoadCrate(addFile func(name, src string) int) (*ast.Crate, error) {
	src, err := fs.ReadFile(FS, RootPath)
	if err != nil {
		return nil, err
	}
	c, err := parser.ParseCrate(addFile(RootPath, string(src)), string(src))
	if err != nil {
		return nil, err
	}
	if err := loadMods(addFile, c.Items, path.Dir(RootPath)); err != nil {
		return nil, err
	}
	return c, nil
}

// loadMods fills the out-of-line modules among items. dir is the directory
// in which child module files of these items live.
func loadMods(addFile func(name, src string) int, items []*ast.Item, dir string) error {
	for _, it := range items {
		m, ok := it.Kind.(*ast.ModItem)
		if !ok {
			continue
		}
		name := it.Name.Name
		if m.Inline {
			if err := loadMods(addFile, m.Items, path.Join(dir, name)); err != nil {
				return err
			}
			continue
		}
		if m.Loaded {
			continue
		}
		var file, childDir string
		var src []byte
		cands := []string{path.Join(dir, name+".rs"), path.Join(dir, name, "mod.rs")}
		for _, cand := range cands {
			if b, err := fs.ReadFile(FS, cand); err == nil {
				file, src = cand, b
				break
			}
		}
		if file == "" {
			return fmt.Errorf("stdlib: file not found for module `%s` (tried %s)", name, strings.Join(cands, ", "))
		}
		childDir = path.Join(dir, name)
		c, err := parser.ParseCrate(addFile(file, string(src)), string(src))
		if err != nil {
			return err
		}
		m.Items = c.Items
		m.Loaded = true
		if err := loadMods(addFile, m.Items, childDir); err != nil {
			return err
		}
	}
	return nil
}
