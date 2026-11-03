package driver

import (
	"os"
	"path/filepath"
	"testing"
)

func TestCCCache(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("RUSTGO_CACHE_DIR", dir)
	t.Setenv("RUSTGO_CC_CACHE", "")
	c := newCCCache("sh", []string{"-O1"})
	if c == nil {
		t.Skip("no sh in PATH")
	}
	obj := filepath.Join(dir, "a.o")
	out := filepath.Join(dir, "b.o")
	if c.get("int x;", out) {
		t.Fatal("hit in an empty cache")
	}
	os.WriteFile(obj, []byte("object"), 0o644)
	c.add("int x;", obj)
	if !c.get("int x;", out) {
		t.Fatal("miss after add")
	}
	if b, _ := os.ReadFile(out); string(b) != "object" {
		t.Fatalf("got %q", b)
	}
	if c.get("int y;", out) {
		t.Fatal("hit for another source")
	}
	// other flags: another key
	if c2 := newCCCache("sh", []string{"-O2"}); c2.get("int x;", out) {
		t.Fatal("hit with other flags")
	}
	// pruning to 0 bytes empties the cache
	t.Setenv("RUSTGO_CC_CACHE_MB", "0")
	c.prune()
	if c.get("int x;", out) {
		t.Fatal("hit after pruning")
	}
	t.Setenv("RUSTGO_CC_CACHE", "0")
	if newCCCache("sh", nil) != nil {
		t.Fatal("cache not disabled")
	}
}
