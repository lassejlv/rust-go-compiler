#!/usr/bin/env bash
# Regenerate expected outputs (.stdout / .exitcode) for tests/run-pass/*.rs
# using the real rustc as the oracle.
#
# Usage: tests/gen_expected.sh [pattern]
#   pattern: optional substring; only tests whose file name contains it are regenerated.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TESTDIR="$ROOT/tests/run-pass"
OUT="$ROOT/tests/out/rustc"
mkdir -p "$OUT"
PATTERN="${1:-}"
RUSTC="${RUSTC:-rustc}"

fail=0
count=0
for src in "$TESTDIR"/*.rs; do
    name="$(basename "$src" .rs)"
    if [ -n "$PATTERN" ] && [[ "$name" != *"$PATTERN"* ]]; then
        continue
    fi
    count=$((count + 1))
    bin="$OUT/$name"
    if ! "$RUSTC" --edition 2021 -A warnings -o "$bin" "$src" 2> "$OUT/$name.rustc.err"; then
        echo "RUSTC FAILED: $name"
        sed -n '1,20p' "$OUT/$name.rustc.err"
        fail=1
        continue
    fi
    (cd "$OUT" && timeout 10 "$bin" > "$TESTDIR/$name.stdout" 2> "$OUT/$name.stderr")
    code=$?
    if [ "$code" -ne 0 ]; then
        echo "$code" > "$TESTDIR/$name.exitcode"
        echo "$name: exit code $code"
    else
        rm -f "$TESTDIR/$name.exitcode"
    fi
done
echo "generated expected output for $count test(s)"
exit $fail
