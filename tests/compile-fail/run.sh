#!/usr/bin/env bash
# Compile-fail test runner for the rustgo compiler.
#
# Usage: tests/compile-fail/run.sh [-q] [-j N] [pattern]
#
#   pattern     optional substring; only tests whose name contains it are run
#   -q          quiet: only print failures and the summary line
#   -j N        number of parallel jobs (default: $JOBS or 4)
#
# Environment: RUSTGO=path overrides the compiler executable (default ./rustgo,
# relative to the repository root), JOBS, COMPILE_TIMEOUT (default 60),
# OUTDIR (default /tmp/rustgo-compile-fail).
#
# Every tests/compile-fail/*.rs program is invalid Rust: rustc rejects it with
# the error named on its first line (`//~ ERROR E0XXX`, or `//~ ERROR lifetime`
# for borrowck's code-less "lifetime may not live long enough").
# A test PASSES when rustgo exits non-zero and prints a line containing
# `error`; it FAILS when rustgo accepts the program, when only the C backend
# (gcc/ld) reports an error, or when rustgo crashes/times out without
# reporting an error. The expected error code is shown for failures
# but is not required to match.
# Results are printed in test order, followed by `passed X / Y`.
# Exit status: 0 if every selected test passed, 1 otherwise.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TESTDIR="tests/compile-fail"
OUTDIR="${OUTDIR:-/tmp/rustgo-compile-fail}"
COMPILE_TIMEOUT="${COMPILE_TIMEOUT:-60}"
RUSTGO="${RUSTGO:-./rustgo}"

# ---------------------------------------------------------------------------
# Worker mode: compile a single test and write its result file.
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--worker" ]; then
    cd "$ROOT" || exit 2
    name="$2"
    src="$TESTDIR/$name.rs"
    bin="$OUTDIR/$name"
    res="$OUTDIR/$name.result"
    clog="$OUTDIR/$name.compile.log"
    rm -f "$bin" "$res" "$clog"
    expected="$(head -n 1 "$src" | sed -n 's#^//~ ERROR \(.*\)$#\1#p')"

    timeout -k 5 "$COMPILE_TIMEOUT" "$RUSTGO" "$src" -o "$bin" > "$clog" 2>&1
    rc=$?
    if [ $rc -eq 124 ] || [ $rc -eq 137 ]; then
        echo "FAIL compile timeout (${COMPILE_TIMEOUT}s), expected $expected" > "$res"
    elif [ $rc -eq 0 ]; then
        echo "FAIL accepted (expected $expected)" > "$res"
    elif grep -qiE 'panic:|goroutine [0-9]+ \[|internal compiler error' "$clog"; then
        { echo "FAIL compiler crash (exit $rc), expected $expected"; head -n 3 "$clog"; } > "$res"
    elif grep -i 'error' "$clog" | grep -qvE '\.(c|h):[0-9]+(:[0-9]+)?: (fatal )?error|^collect2|ld returned'; then
        { echo "PASS"; grep -i 'error' "$clog" | grep -vE '\.(c|h):[0-9]+' | head -n 1; } > "$res"
    elif grep -qi 'error' "$clog"; then
        # Only the C compiler/linker complained: the rustgo front end accepted it.
        { echo "FAIL accepted by front end, C backend failed (expected $expected)"; grep -i -m 1 'error' "$clog"; } > "$res"
    else
        { echo "FAIL exit $rc without an error message, expected $expected"; head -n 3 "$clog"; } > "$res"
    fi
    rm -f "$bin"
    exit 0
fi

# ---------------------------------------------------------------------------
# Driver mode
# ---------------------------------------------------------------------------
QUIET=0
JOBS="${JOBS:-4}"
PATTERN=""
while [ $# -gt 0 ]; do
    case "$1" in
        -q) QUIET=1 ;;
        -j) shift; JOBS="${1:-4}" ;;
        -j*) JOBS="${1#-j}" ;;
        -h|--help)
            sed -n '2,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        -*) echo "unknown option: $1" >&2; exit 2 ;;
        *) PATTERN="$1" ;;
    esac
    shift
done

cd "$ROOT" || exit 2
mkdir -p "$OUTDIR"
if [ ! -x "$RUSTGO" ]; then
    echo "no $RUSTGO executable found (build it with: go build -o rustgo ./cmd/rustgo)"
    exit 1
fi

names=()
for src in "$TESTDIR"/*.rs; do
    [ -e "$src" ] || continue
    name="$(basename "$src" .rs)"
    if [ -n "$PATTERN" ] && [[ "$name" != *"$PATTERN"* ]]; then
        continue
    fi
    names+=("$name")
done

total=${#names[@]}
if [ "$total" -eq 0 ]; then
    echo "no tests match '${PATTERN}'"
    echo "passed 0 / 0"
    exit 1
fi

export COMPILE_TIMEOUT RUSTGO OUTDIR
printf '%s\n' "${names[@]}" | xargs -P "$JOBS" -n 1 bash "${BASH_SOURCE[0]}" --worker

passed=0
for name in "${names[@]}"; do
    res="$OUTDIR/$name.result"
    if [ ! -f "$res" ]; then
        echo "FAIL $name: runner error (no result file)"
        continue
    fi
    status="$(head -n 1 "$res")"
    if [ "$status" = "PASS" ]; then
        passed=$((passed + 1))
        if [ $QUIET -eq 0 ]; then
            echo "PASS $name"
            tail -n +2 "$res" | head -n 1 | sed 's/^/    /'
        fi
    else
        echo "FAIL $name: ${status#FAIL }"
        tail -n +2 "$res" | sed 's/^/    /'
    fi
done

echo "passed $passed / $total"
[ "$passed" -eq "$total" ]
