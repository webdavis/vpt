fmt:
  cargo fmt --all -- --check

clippy:
  cargo clippy --locked --workspace --all-targets --features dev-tools -- -D warnings

test:
  cargo test --locked --workspace --no-fail-fast --features dev-tools

test-rust: test

lint-check: fmt clippy

doc:
  RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps

gitleaks:
  gitleaks git --no-banner

file-size:
  #!/usr/bin/env bash
  set -euo pipefail
  status=0
  while IFS= read -r f; do
    read -r impl total < <(awk '
      /^[[:space:]]*#\[cfg\(test\)\]/ && !seen { seen = 1 }
      !seen { impl++ }
      { total++ }
      END {
        if (FILENAME ~ /(^|\/)tests(\.rs|\/)/) impl = 0
        printf "%d %d\n", impl, total
      }' "$f")
    if (( total > 500 )); then
      printf 'FAIL %5d total %s\n' "$total" "$f"
      status=1
    elif (( total > 300 || impl > 200 )); then
      printf 'WARN %5d impl %5d total %s\n' "$impl" "$total" "$f"
    fi
  done < <(git -c core.fsmonitor=false ls-files 'crates/*.rs' 'crates/**/*.rs')
  exit "$status"

ship: fmt clippy test doc gitleaks file-size
