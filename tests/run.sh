#!/usr/bin/env bash
# End-to-end test runner for the rustgo compiler.
#
# Usage: tests/run.sh [-q] [-j N] [--no-build] [pattern]
#
#   pattern     optional substring; only tests whose name contains it are run
#   -q          quiet: only print failures and the summary line
#   -j N        number of parallel jobs (default: $JOBS or 4)
#   --no-build  do not run `go build` first (use the existing ./rustgo)
#
# Environment: RUSTGO=path overrides the compiler executable (default ./rustgo;
# implies nothing about building), JOBS, COMPILE_TIMEOUT, RUN_TIMEOUT, OUTDIR
# (default tests/out; use a separate one when running suites concurrently).
#
# For every tests/run-pass/NNN_name.rs (matching the pattern) this:
#   1. compiles it with `./rustgo file.rs -o tests/out/NNN_name`   (timeout 60s)
#   2. runs the produced binary                                    (timeout 10s)
#   3. compares stdout with NNN_name.stdout and the exit code with
#      NNN_name.exitcode (0 when that file does not exist).
# Optional NNN_name.flags holds extra compiler flags (e.g. --test; harness
# timings are then normalized) and NNN_name.args the program's arguments.
# Results are printed in test order, followed by `passed X / Y`.
# Exit status: 0 if every selected test passed, 1 otherwise.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TESTDIR="tests/run-pass"
OUTDIR="${OUTDIR:-tests/out}"  # set OUTDIR to run several suites concurrently
COMPILE_TIMEOUT="${COMPILE_TIMEOUT:-60}"
RUN_TIMEOUT="${RUN_TIMEOUT:-10}"
RUSTGO="${RUSTGO:-./rustgo}"

# ---------------------------------------------------------------------------
# Worker mode: run a single test and write its result file.
#   result file line 1: PASS | FAIL <reason>
#   following lines:    details (indented when printed)
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--worker" ]; then
    cd "$ROOT" || exit 2
    name="$2"
    src="$TESTDIR/$name.rs"
    bin="$OUTDIR/$name"
    res="$OUTDIR/$name.result"
    clog="$OUTDIR/$name.compile.log"
    actual="$OUTDIR/$name.actual"
    stderr_f="$OUTDIR/$name.stderr"
    rm -f "$bin" "$res" "$clog" "$actual" "$stderr_f"

    flags=()
    if [ -f "$TESTDIR/$name.flags" ]; then
        read -r -a flags < "$TESTDIR/$name.flags"
    fi
    timeout -k 5 "$COMPILE_TIMEOUT" "$RUSTGO" "${flags[@]}" "$src" -o "$bin" > "$clog" 2>&1
    rc=$?
    if [ $rc -eq 124 ] || [ $rc -eq 137 ]; then
        { echo "FAIL compile timeout (${COMPILE_TIMEOUT}s)"; head -n 5 "$clog"; } > "$res"
        exit 0
    fi
    if [ $rc -ne 0 ] || [ ! -x "$bin" ]; then
        {
            if [ $rc -ne 0 ]; then
                echo "FAIL compile error (exit $rc)"
            else
                echo "FAIL compile error (no executable produced)"
            fi
            head -n 5 "$clog"
        } > "$res"
        exit 0
    fi

    expected_code=0
    if [ -f "$TESTDIR/$name.exitcode" ]; then
        expected_code="$(tr -d '[:space:]' < "$TESTDIR/$name.exitcode")"
    fi

    rargs=()
    if [ -f "$TESTDIR/$name.args" ]; then
        read -r -a rargs < "$TESTDIR/$name.args"
    fi
    (cd "$OUTDIR" && timeout -k 2 "$RUN_TIMEOUT" "./$name" "${rargs[@]}" > "$name.actual" 2> "$name.stderr" < /dev/null)
    code=$?
    if [ -f "$TESTDIR/$name.flags" ]; then
        # test harness output carries wall-clock timings
        sed -i -E -e 's/finished in [0-9]+\.[0-9]+s/finished in 0.00s/' -e "s/^(thread '[^']*') \([0-9]+\) panicked/\1 panicked/" "$actual"
    fi
    if { [ $code -eq 124 ] || [ $code -eq 137 ]; } && [ "$expected_code" != "$code" ]; then
        {
            echo "FAIL runtime timeout (${RUN_TIMEOUT}s)"
            tail -n 3 "$actual" | sed 's/^/stdout: /'
        } > "$res"
        exit 0
    fi

    out_ok=1
    if ! cmp -s "$TESTDIR/$name.stdout" "$actual"; then
        out_ok=0
    fi
    if [ "$code" = "$expected_code" ] && [ $out_ok -eq 1 ]; then
        echo "PASS" > "$res"
        exit 0
    fi
    {
        if [ "$code" != "$expected_code" ] && [ $out_ok -eq 0 ]; then
            echo "FAIL exit code $code (expected $expected_code) and output mismatch"
        elif [ "$code" != "$expected_code" ]; then
            echo "FAIL runtime exit code $code (expected $expected_code)"
        else
            echo "FAIL output mismatch"
        fi
        if [ "$code" != "$expected_code" ] && [ -s "$stderr_f" ]; then
            head -n 3 "$stderr_f" | sed 's/^/stderr: /'
        fi
        if [ $out_ok -eq 0 ]; then
            diff -u --label expected --label actual "$TESTDIR/$name.stdout" "$actual" | head -n 20
        fi
    } > "$res"
    exit 0
fi

# ---------------------------------------------------------------------------
# Driver mode
# ---------------------------------------------------------------------------
QUIET=0
BUILD=1
JOBS="${JOBS:-4}"
PATTERN=""
while [ $# -gt 0 ]; do
    case "$1" in
        -q) QUIET=1 ;;
        -j) shift; JOBS="${1:-4}" ;;
        -j*) JOBS="${1#-j}" ;;
        --no-build) BUILD=0 ;;
        -h|--help)
            sed -n '2,21p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        -*) echo "unknown option: $1" >&2; exit 2 ;;
        *) PATTERN="$1" ;;
    esac
    shift
done

cd "$ROOT" || exit 2
mkdir -p "$OUTDIR"

if [ $BUILD -eq 1 ]; then
    if ! go build -o rustgo ./cmd/rustgo > "$OUTDIR/build.log" 2>&1; then
        echo "BUILD FAILED: go build -o rustgo ./cmd/rustgo"
        head -n 30 "$OUTDIR/build.log"
        exit 1
    fi
fi
if [ ! -x "$RUSTGO" ]; then
    echo "no $RUSTGO executable found (build it or drop --no-build)"
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

export COMPILE_TIMEOUT RUN_TIMEOUT RUSTGO
printf '%s\n' "${names[@]}" | xargs -P "$JOBS" -n 1 bash "${BASH_SOURCE[0]}" --worker

passed=0
failed_names=()
for name in "${names[@]}"; do
    res="$OUTDIR/$name.result"
    if [ ! -f "$res" ]; then
        echo "FAIL $name: runner error (no result file)"
        failed_names+=("$name")
        continue
    fi
    status="$(head -n 1 "$res")"
    if [ "$status" = "PASS" ]; then
        passed=$((passed + 1))
        [ $QUIET -eq 1 ] || echo "PASS $name"
    else
        failed_names+=("$name")
        echo "FAIL $name: ${status#FAIL }"
        tail -n +2 "$res" | sed 's/^/    /'
    fi
done

echo "passed $passed / $total"
[ "$passed" -eq "$total" ]
