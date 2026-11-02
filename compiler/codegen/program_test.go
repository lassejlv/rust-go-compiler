package codegen

import "testing"

// The runtime must stay compilable as several translation units: mutable
// globals go through RG_GLOBAL, never `static` (see runtime.h).
func TestRuntimeSplittable(t *testing.T) {
	if p := runtimeSplitProblem(runtimeH + "\n" + coroH); p != "" {
		t.Fatalf("runtime.h/coro.h declare static data (use RG_GLOBAL / RG_INIT): %s", p)
	}
	for _, bad := range []string{
		"static int x;",
		"static __thread int y = 0;",
		"static char buf[16];",
		"static int f(void) { static int n; return n++; }",
		"  static int n;",
	} {
		if runtimeSplitProblem(bad) == "" {
			t.Errorf("not detected: %s", bad)
		}
	}
	for _, ok := range []string{
		"static int f(void);",
		"__attribute__((noinline)) static void g(int x) { return; }",
		"static const char tbl[] = \"abc\";",
		"static inline int h(int a) { return a; }",
	} {
		if p := runtimeSplitProblem(ok); p != "" {
			t.Errorf("false positive: %s", ok)
		}
	}
}
