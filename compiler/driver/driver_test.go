package driver

import (
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// TestMinStd compiles programs against the minimal bring-up std and
// compares their output with rustc's.
func TestMinStd(t *testing.T) {
	if _, err := exec.LookPath("gcc"); err != nil {
		t.Skip("gcc not available")
	}
	files, _ := filepath.Glob("../../tests/minstd-pass/*.rs")
	for _, f := range files {
		f := f
		t.Run(filepath.Base(f), func(t *testing.T) {
			out := filepath.Join(t.TempDir(), "bin")
			opts := &Options{Input: f, Output: out, StdDir: "../../tests/minstd"}
			if err := Compile(opts); err != nil {
				t.Fatalf("compile: %v", err)
			}
			cmd := exec.Command(out)
			got, _ := cmd.Output()
			code := cmd.ProcessState.ExitCode()
			base := strings.TrimSuffix(f, ".rs")
			want, _ := os.ReadFile(base + ".stdout")
			wantCode := 0
			if b, err := os.ReadFile(base + ".exitcode"); err == nil {
				wantCode, _ = strconv.Atoi(strings.TrimSpace(string(b)))
			}
			if string(got) != string(want) || code != wantCode {
				t.Errorf("exit %d (want %d)\n--- got ---\n%s\n--- want ---\n%s", code, wantCode, got, want)
			}
		})
	}
}
