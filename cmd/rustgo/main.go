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
	"runtime"
	"runtime/debug"
	"runtime/pprof"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"rustgo/compiler/driver"
)

// reportMemory prints the peak memory use of the compiler (with -v).
func reportMemory() {
	var ru syscall.Rusage
	if syscall.Getrusage(syscall.RUSAGE_SELF, &ru) == nil {
		fmt.Fprintf(os.Stderr, "max RSS %d MB\n", ru.Maxrss>>10)
	}
}

func usage() {
	fmt.Fprintf(os.Stderr, `rustgo - a Rust compiler written in Go

usage:
  rustgo [options] <file.rs>          compile to an executable
  rustgo run [options] <file.rs> [-- args...]  compile and run

options:
  -o <path>         output executable (default: input name without .rs)
  --emit-c <path>   also write the generated C source
  --keep-c          keep the temporary C file
  --test            build the test harness (#[test] functions; rustc --test)
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
	if os.Getenv("GOGC") == "" {
		debug.SetGCPercent(800)
	}
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
	if p := os.Getenv("RUSTGO_MEMPROFILE"); p != "" {
		// allocation profile (go tool pprof -sample_index=alloc_space)
		defer func() {
			if f, err := os.Create(p); err == nil {
				runtime.GC() // (the profile is as of the last GC)
				pprof.Lookup("allocs").WriteTo(f, 0)
				f.Close()
			}
		}()
	}
	args := os.Args[1:]
	if len(args) == 0 {
		usage()
		os.Exit(1)
	}
	run := false
	test := false
	if args[0] == "run" {
		run = true
		args = args[1:]
	} else if args[0] == "test" && (len(args) == 1 || !strings.HasSuffix(args[1], ".rs")) {
		// in a cargo project: build and run the test harnesses
		cargoTest(args[1:])
		return
	} else if args[0] == "test" {
		run, test = true, true
		args = args[1:]
	}
	if args[0] == "build" {
		cargoBuild(args[1:])
		return
	}
	opts := &driver.Options{Test: test}
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
		case a == "--test":
			opts.Test = true
		case a == "--release":
			opts.Release = true
		case a == "-v":
			opts.Verbose = true
		case a == "-h" || a == "--help":
			usage()
			return
		case strings.HasPrefix(a, "-") && opts.Input != "" && run:
			// options of the program run (e.g. the test harness's --exact)
			runArgs = append(runArgs, a)
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
	if opts.Verbose {
		reportMemory()
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
	if opts.Verbose {
		reportMemory()
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

// cargoTest implements `rustgo test` in a Cargo package (like `cargo
// test`): builds the library, binaries and integration tests as test
// harnesses and runs them. Arguments after `--` (and a bare filter) go to
// the harnesses.
func cargoTest(args []string) {
	opts := &driver.Options{}
	co := &driver.CargoOptions{}
	var sel, harness []string
	noRun, noFailFast := false, false
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch {
		case a == "--":
			harness = append(harness, args[i+1:]...)
			i = len(args)
		case a == "--manifest-path":
			i++
			co.Manifest = args[i]
		case a == "--lib":
			sel = append(sel, "lib")
		case a == "--bins":
			sel = append(sel, "bins")
		case a == "--doc":
			sel = append(sel, "doc")
		case a == "--tests":
			sel = append(sel, "lib", "bins", "tests")
		case a == "--bin" || a == "--test":
			i++
			sel = append(sel, strings.TrimPrefix(a, "--")+":"+args[i])
		case a == "--release" || a == "-r":
			opts.Release = true
		case a == "-v":
			opts.Verbose = true
		case a == "--no-run":
			noRun = true
		case a == "--no-fail-fast":
			noFailFast = true
		case a == "--keep-c":
			opts.KeepC = true
		case strings.HasPrefix(a, "-"):
			fmt.Fprintf(os.Stderr, "error: unknown test option %s\n", a)
			os.Exit(1)
		default:
			harness = append([]string{a}, harness...)
		}
	}
	outDir, err := os.MkdirTemp("", "rustgo-test-")
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	defer os.RemoveAll(outDir)
	targets, err := driver.BuildCargoTests(opts, co, outDir, sel)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(101)
	}
	if noRun {
		return
	}
	var failed []string
	for _, t := range targets {
		if t.Kind == "doc" {
			fmt.Fprintf(os.Stderr, "   Doc-tests %s\n", t.Name)
			if !runDocTests(t, harness) {
				failed = append(failed, "`--doc`")
				if !noFailFast {
					fmt.Fprintln(os.Stderr, "error: doctest failed, to rerun pass `--doc`")
					os.Exit(101)
				}
			}
			continue
		}
		what := "unittests " + t.Src
		if t.Kind == "test" {
			what = t.Src
		}
		fmt.Fprintf(os.Stderr, "     Running %s (%s)\n", what, filepath.Base(t.Exe))
		cmd := exec.Command(t.Exe, harness...)
		cmd.Dir = t.Dir
		cmd.Env = append(os.Environ(), "CARGO_MANIFEST_DIR="+t.Dir)
		cmd.Stdin, cmd.Stdout, cmd.Stderr = os.Stdin, os.Stdout, os.Stderr
		if err := cmd.Run(); err != nil {
			flag := "--" + t.Kind
			if t.Kind != "lib" {
				flag += " " + t.Name
			}
			failed = append(failed, "`"+flag+"`")
			if !noFailFast {
				fmt.Fprintf(os.Stderr, "error: test failed, to rerun pass `%s`\n", flag)
				os.Exit(101)
			}
		}
	}
	if len(failed) > 0 {
		fmt.Fprintf(os.Stderr, "error: %d target(s) failed:\n", len(failed))
		for _, f := range failed {
			fmt.Fprintf(os.Stderr, "    %s\n", f)
		}
		os.Exit(101)
	}
}

// runDocTests runs a library's doc-tests and prints libtest-style results;
// false if any failed.
func runDocTests(t driver.TestTarget, args []string) bool {
	var filters, skips []string
	exact, ignored, includeIgnored, list := false, false, false, false
	for i := 0; i < len(args); i++ {
		switch a := args[i]; {
		case a == "--exact":
			exact = true
		case a == "--ignored":
			ignored = true
		case a == "--include-ignored":
			includeIgnored = true
		case a == "--list":
			list = true
		case a == "--skip" && i+1 < len(args):
			i++
			skips = append(skips, args[i])
		case strings.HasPrefix(a, "--test-threads") || a == "--nocapture" || a == "--no-capture" || a == "-q" || a == "--quiet":
			if a == "--test-threads" {
				i++
			}
		case !strings.HasPrefix(a, "-"):
			filters = append(filters, a)
		}
	}
	match := func(name, f string) bool {
		if exact {
			return name == f
		}
		return strings.Contains(name, f)
	}
	var run []*driver.DocTest
	filtered := 0
	for _, dt := range t.DocTests {
		ok := len(filters) == 0
		for _, f := range filters {
			ok = ok || match(dt.Name, f)
		}
		for _, f := range skips {
			if match(dt.Name, f) {
				ok = false
			}
		}
		if ignored && !dt.Ignore {
			ok = false
		}
		if !ok {
			filtered++
			continue
		}
		run = append(run, dt)
	}
	if list {
		for _, dt := range run {
			fmt.Printf("%s: test\n", dt.Name)
		}
		fmt.Printf("\n%d tests, 0 benchmarks\n", len(run))
		return true
	}
	start := time.Now()
	plural := "s"
	if len(run) == 1 {
		plural = ""
	}
	fmt.Printf("\nrunning %d test%s\n", len(run), plural)
	type res struct {
		status string // ok, FAILED, ignored
		out    string
	}
	results := make([]res, len(run))
	var wg sync.WaitGroup
	sem := make(chan struct{}, runtime.NumCPU())
	for i, dt := range run {
		if dt.Ignore && !ignored && !includeIgnored {
			results[i] = res{status: "ignored"}
			continue
		}
		wg.Add(1)
		go func(i int, dt *driver.DocTest) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			results[i] = runDocTest(dt, t.Dir)
		}(i, dt)
	}
	wg.Wait()
	passed, failedN, ign := 0, 0, 0
	var failures []int
	for i, dt := range run {
		suffix := ""
		if dt.CompileFail {
			suffix = " - compile fail"
		} else if dt.NoRun {
			suffix = " - compile"
		}
		fmt.Printf("test %s%s ... %s\n", dt.Name, suffix, results[i].status)
		switch results[i].status {
		case "ok":
			passed++
		case "ignored":
			ign++
		default:
			failedN++
			failures = append(failures, i)
		}
	}
	if len(failures) > 0 {
		fmt.Printf("\nfailures:\n\n")
		for _, i := range failures {
			fmt.Printf("---- %s stdout ----\n%s\n", run[i].Name, results[i].out)
		}
		fmt.Printf("\nfailures:\n")
		for _, i := range failures {
			fmt.Printf("    %s\n", run[i].Name)
		}
	}
	verdict := "ok"
	if failedN > 0 {
		verdict = "FAILED"
	}
	fmt.Printf("\ntest result: %s. %d passed; %d failed; %d ignored; 0 measured; %d filtered out; finished in %.2fs\n\n",
		verdict, passed, failedN, ign, filtered, time.Since(start).Seconds())
	return failedN == 0
}

func runDocTest(dt *driver.DocTest, dir string) (r struct {
	status string
	out    string
}) {
	r.status = "FAILED"
	switch {
	case dt.CompileFail:
		if dt.BuildErr != "" {
			r.status = "ok"
		} else {
			r.out = "Test compiled successfully, but it's marked `compile_fail`."
		}
		return
	case dt.BuildErr != "":
		r.out = dt.BuildErr + "\nCouldn't compile the test."
		return
	case dt.NoRun:
		r.status = "ok"
		return
	}
	cmd := exec.Command(dt.Exe, strconv.Itoa(dt.Index))
	cmd.Dir = dir
	var so, se strings.Builder
	cmd.Stdout, cmd.Stderr = &so, &se
	err := cmd.Run()
	if err == nil {
		if dt.ShouldPanic {
			r.out = "Test executable succeeded, but it's marked `should_panic`."
		} else {
			r.status = "ok"
		}
		return
	}
	if dt.ShouldPanic {
		r.status = "ok"
		return
	}
	reason := err.Error()
	if ee, ok := err.(*exec.ExitError); ok && ee.Exited() {
		reason = fmt.Sprintf("exit status: %d", ee.ExitCode())
	}
	var b strings.Builder
	fmt.Fprintf(&b, "Test executable failed (%s).\n", reason)
	if so.Len() > 0 {
		fmt.Fprintf(&b, "\nstdout:\n%s", so.String())
	}
	if se.Len() > 0 {
		fmt.Fprintf(&b, "\nstderr:\n%s\n", se.String())
	}
	r.out = b.String()
	return
}
