package ast

import (
	"bytes"
	"os"
	"os/exec"
	"testing"
)

// TestSerialGenUpToDate checks that serial_gen.go matches the AST type
// declarations (run `go generate ./compiler/ast` after changing them).
func TestSerialGenUpToDate(t *testing.T) {
	if _, err := exec.LookPath("go"); err != nil {
		t.Skip("go tool not available")
	}
	out, err := exec.Command("go", "run", "gen_serial.go", "-stdout").Output()
	if err != nil {
		t.Fatalf("gen_serial: %v", err)
	}
	cur, err := os.ReadFile("serial_gen.go")
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(out, cur) {
		t.Fatalf("serial_gen.go is stale: run `go generate ./compiler/ast`")
	}
}
