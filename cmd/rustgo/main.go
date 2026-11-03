// Command rustgo is a Rust compiler written in Go.
//
// Usage:
//
//	rustgo [options] <file.rs> [-o output]
//	rustgo run <file.rs> [args...]
package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime/debug"
	"runtime/pprof"
	"strings"

	"rustgo/compiler/driver"
)

func usage() {
	fmt.Fprintf(os.Stderr, `rustgo - a Rust compiler written in Go

usage:
  rustgo [options] <file.rs>          compile to an executable
  rustgo run [options] <file.rs> [-- args...]  compile and run

options:
  -o <path>         output executable (default: input name without .rs)
  --emit-c <path>   also write the generated C source
  --keep-c          keep the temporary C file
  --check           only check the program (type and borrow checking), no output
  --release         optimize (gcc -O2), no integer overflow checks (like cargo --release)
  --std <dir>       use a std library from a directory instead of the built-in one
  --cc <compiler>   C compiler to use (default: gcc, or $RUSTGO_CC)
  -O <level>        C optimization level (default 1)
  --edition <ed>    Rust edition of the input (2015, 2018, 2021 default, 2024)
  -v                verbose timings
`)
}

func main() {
	// The compiler is short-lived: trade memory for less GC time.
	debug.SetGCPercent(800)
	if os.Getenv("GOMEMLIMIT") == "" {
		// but collect before the heap grows beyond a few GB (a large
		// live heap times 9 would exhaust the machine's memory)
		debug.SetMemoryLimit(2 << 30)
	}
	if p := os.Getenv("RUSTGO_CPUPROFILE"); p != "" {
		f, err := os.Create(p)
		if err == nil {
			pprof.StartCPUProfile(f)
			defer pprof.StopCPUProfile()
		}
	}
	args := os.Args[1:]
	if len(args) == 0 {
		usage()
		os.Exit(1)
	}
	run := false
	if args[0] == "run" {
		run = true
		args = args[1:]
	}
	if args[0] == "build" {
		cargoBuild(args[1:])
		return
	}
	opts := &driver.Options{}
	var runArgs []string
	for i := 0; i < len(args); i++ {
		a := args[i]
		next := func() string {
			if i+1 >= len(args) {
				fmt.Fprintf(os.Stderr, "error: missing value for %s\n", a)
				os.Exit(1)
			}
			i++
			return args[i]
		}
		switch {
		case a == "--":
			runArgs = args[i+1:]
			i = len(args)
		case a == "-o":
			opts.Output = next()
		case a == "--emit-c":
			opts.EmitC = next()
		case a == "--keep-c":
			opts.KeepC = true
		case a == "--std":
			opts.StdDir = next()
		case a == "--cc":
			opts.CC = next()
		case a == "-O":
			opts.OptLevel = next()
		case strings.HasPrefix(a, "-O") && len(a) > 2:
			opts.OptLevel = a[2:]
		case a == "--edition":
			opts.Edition = next()
		case strings.HasPrefix(a, "--edition="):
			opts.Edition = strings.TrimPrefix(a, "--edition=")
		case a == "--check-all":
			opts.CheckAll = true
		case a == "--check":
			opts.Check = true
		case a == "--release":
			opts.Release = true
		case a == "-v":
			opts.Verbose = true
		case a == "-h" || a == "--help":
			usage()
			return
		case strings.HasPrefix(a, "-"):
			fmt.Fprintf(os.Stderr, "error: unknown option %s\n", a)
			os.Exit(1)
		default:
			if opts.Input != "" {
				runArgs = append(runArgs, a)
				continue
			}
			opts.Input = a
		}
	}
	if opts.Input == "" {
		usage()
		os.Exit(1)
	}
	if opts.Output == "" {
		if run {
			f, err := os.CreateTemp("", "rustgo-bin-*")
			if err != nil {
				fmt.Fprintln(os.Stderr, err)
				os.Exit(1)
			}
			f.Close()
			opts.Output = f.Name()
			defer os.Remove(opts.Output)
		} else {
			opts.Output = strings.TrimSuffix(filepath.Base(opts.Input), ".rs")
		}
	}
	if opts.CheckAll {
		if err := driver.CheckAll(opts); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		return
	}
	if err := driver.Compile(opts); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if run {
		cmd := exec.Command(opts.Output, runArgs...)
		cmd.Stdin, cmd.Stdout, cmd.Stderr = os.Stdin, os.Stdout, os.Stderr
		if err := cmd.Run(); err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				os.Remove(opts.Output)
				os.Exit(ee.ExitCode())
			}
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
	}
}

// cargoBuild implements `rustgo build [--manifest-path P] [--bin N] [-o out]`.
func cargoBuild(args []string) {
	opts := &driver.Options{}
	co := &driver.CargoOptions{}
	runAfter := false
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--manifest-path":
			i++
			co.Manifest = args[i]
		case "--bin":
			i++
			co.Bin = args[i]
		case "-o":
			i++
			opts.Output = args[i]
		case "--emit-c":
			i++
			opts.EmitC = args[i]
		case "--keep-c":
			opts.KeepC = true
		case "-v":
			opts.Verbose = true
		case "--run":
			runAfter = true
		case "--release", "-r":
			opts.Release = true
		default:
			fmt.Fprintf(os.Stderr, "error: unknown build option %s\n", args[i])
			os.Exit(1)
		}
	}
	if opts.Output == "" {
		opts.Output = "a.out"
	}
	if err := driver.BuildCargo(opts, co); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if runAfter {
		cmd := exec.Command(opts.Output)
		cmd.Stdin, cmd.Stdout, cmd.Stderr = os.Stdin, os.Stdout, os.Stderr
		if err := cmd.Run(); err != nil {
			if ee, ok := err.(*exec.ExitError); ok {
				os.Exit(ee.ExitCode())
			}
		}
	}
}
