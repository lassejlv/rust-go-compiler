package driver

import (
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
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
		t.Fatal("decoded std crate differs: ", firstDiff(reflect.ValueOf(c), reflect.ValueOf(c2), "crate"))
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

// firstDiff returns the path of the first difference between a and b.
func firstDiff(a, b reflect.Value, path string) string {
	if a.Kind() != b.Kind() {
		return path + ": kind"
	}
	switch a.Kind() {
	case reflect.Ptr, reflect.Interface:
		if a.IsNil() || b.IsNil() {
			if a.IsNil() != b.IsNil() {
				return path + ": nil"
			}
			return ""
		}
		if a.Kind() == reflect.Interface && a.Elem().Type() != b.Elem().Type() {
			return path + ": type"
		}
		return firstDiff(a.Elem(), b.Elem(), path)
	case reflect.Struct:
		for i := 0; i < a.NumField(); i++ {
			if d := firstDiff(a.Field(i), b.Field(i), path+"."+a.Type().Field(i).Name); d != "" {
				return d
			}
		}
		return ""
	case reflect.Slice:
		if a.IsNil() != b.IsNil() || a.Len() != b.Len() {
			return path + ": len/nil"
		}
		for i := 0; i < a.Len(); i++ {
			if d := firstDiff(a.Index(i), b.Index(i), path+"["+strconv.Itoa(i)+"]"); d != "" {
				return d
			}
		}
		return ""
	case reflect.Func:
		return ""
	}
	if !reflect.DeepEqual(a.Interface(), b.Interface()) {
		return path + ": " + fmt.Sprint(a.Interface(), " != ", b.Interface())
	}
	return ""
}
