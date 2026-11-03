#!/usr/bin/env bash
# Builds each Cargo project under tests/cargo with rustgo and compares output.
set -u
cd "$(dirname "$0")/../.."
go build -o rustgo ./cmd/rustgo || exit 1
fail=0
for spec in "tests/cargo/app expected.stdout" "tests/cargo/app2 expected2.stdout" "tests/cargo/app3 expected3.stdout"; do
  set -- $spec; proj=$1; exp=tests/cargo/$2
  out=$(mktemp)
  if ./rustgo build --manifest-path "$proj/Cargo.toml" -o "$out.bin" 2>"$out.err" && "$out.bin" >"$out"; then
    if diff -u "$exp" "$out" >/dev/null; then echo "PASS $proj"; else echo "FAIL $proj (output)"; fail=1; fi
  else
    echo "FAIL $proj"; head -5 "$out.err"; fail=1
  fi
  rm -f "$out" "$out.bin" "$out.err"
done
# `rustgo test`: unit tests (lib + bin, cfg(test), dev-dependencies) and
# integration tests; timings are normalized
out=$(mktemp)
if (cd tests/cargo/testpkg && ../../../rustgo test 2>"$out.err") | sed -E 's/finished in [0-9.]+s/finished in 0.00s/' >"$out" && diff -u tests/cargo/expected_test.stdout "$out" >/dev/null; then
  echo "PASS tests/cargo/testpkg (rustgo test)"
else
  echo "FAIL tests/cargo/testpkg (rustgo test)"; diff -u tests/cargo/expected_test.stdout "$out" | head -20; head -5 "$out.err"; fail=1
fi
rm -f "$out" "$out.err" tests/cargo/testpkg/Cargo.lock
exit $fail
