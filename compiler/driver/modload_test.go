package driver

import (
	"testing"
	"testing/fstest"

	"rustgo/compiler/ast"
	"rustgo/compiler/expand"
)

// Out-of-line modules whose file depends on the crate configuration
// (`#[cfg_attr(unix, path = "unix.rs")]`) or that are configured out are
// loaded by the expander once cfg is known, not eagerly by the driver.
func TestCfgDependentModLoading(t *testing.T) {
	fsys := fstest.MapFS{
		"main.rs": {Data: []byte(`
#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod sys;
#[cfg(test)]
mod tests;
mod plain;
fn main() {}
`)},
		"sys.rs":   {Data: []byte(`compile_error!("default sys");`)},
		"unix.rs":  {Data: []byte(`pub const UNIX: i32 = 1;`)},
		"plain.rs": {Data: []byte(`pub const PLAIN: i32 = 2;`)},
		// tests.rs is missing: configured out, it must not be loaded
	}
	c, err := LoadCrateFS(fsys, "main.rs", "")
	if err != nil {
		t.Fatalf("load: %v", err)
	}
	if err := expand.New().ExpandCrate(c, "main"); err != nil {
		t.Fatalf("expand: %v", err)
	}
	items := map[string]*ast.ModItem{}
	for _, it := range c.Items {
		if m, ok := it.Kind.(*ast.ModItem); ok {
			items[it.Name.Name] = m
		}
	}
	if items["tests"] != nil {
		t.Errorf("cfg(test) module kept")
	}
	for name, want := range map[string]string{"sys": "UNIX", "plain": "PLAIN"} {
		m := items[name]
		if m == nil || len(m.Items) != 1 || m.Items[0].Name.Name != want {
			t.Errorf("module %s: want item %s", name, want)
		}
	}
}
