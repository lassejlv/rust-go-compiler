#!/usr/bin/env bash
# Builds each Cargo project under tests/cargo with rustgo and compares output.
set -u
cd "$(dirname "$0")/../.."
go build -o rustgo ./cmd/rustgo || exit 1
fail=0
for proj in tests/cargo/app; do
  out=$(mktemp)
  if ./rustgo build --manifest-path "$proj/Cargo.toml" -o "$out.bin" 2>"$out.err" && "$out.bin" >"$out"; then
    if diff -u tests/cargo/expected.stdout "$out" >/dev/null; then echo "PASS $proj"; else echo "FAIL $proj (output)"; fail=1; fi
  else
    echo "FAIL $proj"; head -5 "$out.err"; fail=1
  fi
  rm -f "$out" "$out.bin" "$out.err"
done
exit $fail
