package stdlib

import (
	"io/fs"
	"strings"
	"testing"

	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
)

// TestParseAndExpand checks that every library file parses and that the
// whole crate macro-expands without errors.
func TestParseAndExpand(t *testing.T) {
	loaded := map[string]bool{}
	c, err := LoadCrate(func(name, src string) int {
		loaded[name] = true
		return lexer.AddFile(name, src)
	})
	if err != nil {
		t.Fatalf("load: %v", err)
	}
	if err := expand.New().ExpandCrate(c, "std"); err != nil {
		t.Fatalf("expand: %v", err)
	}
	// Every .rs file in the embedded tree must be reachable from the root.
	err = fs.WalkDir(FS, "lib", func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && strings.HasSuffix(p, ".rs") && !loaded[p] {
			t.Errorf("file %s is not reachable from %s", p, RootPath)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}
