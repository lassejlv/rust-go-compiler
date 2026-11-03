package driver

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"rustgo/compiler/ast"
	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
)

// clearUnserialized zeroes what the std cache does not keep (item source
// tokens, only used as procedural macro input).
func clearUnserialized(c *ast.Crate) {
	c.LoadMod = nil
	ast.Walk(c, func(n any) bool {
		if it, ok := n.(*ast.Item); ok {
			it.Tokens = nil
		}
		return true
	})
	var items func([]*ast.Item)
	items = func(its []*ast.Item) {
		for _, it := range its {
			it.Tokens = nil
			switch k := it.Kind.(type) {
			case *ast.ModItem:
				items(k.Items)
			case *ast.ImplItem:
				items(k.Items)
			case *ast.TraitItem:
				items(k.Items)
			case *ast.ExternBlockItem:
				items(k.Items)
			}
		}
	}
	items(c.Items)
}

// TestStdCacheRoundTrip expands std, writes it to a cache file, reads it
// back (twice: also rebased onto a later global state, as in cargo builds
// that load std several times) and compares.
func TestStdCacheRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "std.bin")
	opts := &Options{}
	idBase, fileBase, hygBase := ast.NextID(), len(lexer.Files), expand.HygieneMarks()
	exp := expand.New()
	c, err := loadStd(opts)
	if err != nil {
		t.Fatal(err)
	}
	if err := exp.ExpandCrate(c, "std"); err != nil {
		t.Fatal(err)
	}
	idEnd, fileEnd, hygEnd := ast.NextID(), len(lexer.Files), expand.HygieneMarks()
	writeStdCache(path, c, exp, idBase, fileBase, hygBase)
	if _, err := os.Stat(path); err != nil {
		t.Fatalf("cache not written: %v", err)
	}
	clearUnserialized(c)

	// decode in the state std was produced in: identical crate
	stdFiles := append([]*lexer.SourceFile{}, lexer.Files[fileBase:fileEnd]...)
	lexer.Files = lexer.Files[:fileBase]
	ast.SetNextID(idBase)
	expand.SetHygieneMarks(hygBase)
	exp2 := expand.New()
	c2 := readStdCache(path, exp2)
	if c2 == nil {
		t.Fatal("cache not readable")
	}
	if ast.NextID() != idEnd || len(lexer.Files) != fileEnd || expand.HygieneMarks() != hygEnd {
		t.Fatalf("state after decoding: ids %d/%d files %d/%d marks %d/%d", ast.NextID(), idEnd, len(lexer.Files), fileEnd, expand.HygieneMarks(), hygEnd)
	}
	for i, f := range stdFiles {
		if g := lexer.Files[fileBase+i]; g.Name != f.Name || g.Src != f.Src {
			t.Fatalf("file %d: got %s", i, g.Name)
		}
	}
	if !reflect.DeepEqual(c, c2) {
		t.Fatal("decoded std crate differs")
	}
	if !exp2.IsCrateMacro("std", "__std_int_common") || exp2.IsCrateMacro("std", "no_such_macro") {
		t.Fatal("std macros missing after decoding")
	}

	// decode again after more IDs, files and marks: rebased
	c3 := readStdCache(path, expand.New())
	if c3 == nil {
		t.Fatal("cache not readable (second load)")
	}
	if c3.Items[0].ID == c.Items[0].ID || c3.Items[0].Span.Lo.File == c.Items[0].Span.Lo.File {
		t.Fatal("second load not rebased")
	}
	if ast.NextID() != idEnd+(idEnd-idBase) || len(lexer.Files) != fileEnd+(fileEnd-fileBase) {
		t.Fatal("bad state after second load")
	}

	// corrupt files are ignored
	data, _ := os.ReadFile(path)
	data[len(data)/2] ^= 0xff
	os.WriteFile(path, data, 0o644)
	n := len(lexer.Files)
	if readStdCache(path, expand.New()) != nil || len(lexer.Files) != n {
		t.Fatal("corrupt cache file accepted")
	}
}
