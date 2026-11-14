fmt:
  cargo fmt --all -- --check

clippy:
  cargo clippy --locked --workspace --all-targets --features dev-tools -- -D warnings

test:
  TMPDIR="$(cd "${TMPDIR:-/tmp}" && pwd -P)/" cargo test --locked --workspace --no-fail-fast --features dev-tools

test-rust: test

swift-format:
  bash helper/vpt-macos/scripts/with-isolated-env.sh swiftformat Sources Tests Package.swift --lint --cache ignore --swift-version 6.0 --max-width 100 --trailing-commas never

swift-lint:
  bash helper/vpt-macos/scripts/with-isolated-env.sh swiftlint lint Sources Tests Package.swift --no-cache --strict

swift-build:
  bash helper/vpt-macos/scripts/with-isolated-env.sh swift build -c release --cache-path .build/isolated/cache --config-path .build/isolated/config --security-path .build/isolated/security --scratch-path .build/isolated/scratch --manifest-cache local --disable-dependency-cache --disable-automatic-resolution --explicit-target-dependency-import-check error

swift-test:
  bash helper/vpt-macos/scripts/with-isolated-env.sh swift test --cache-path .build/isolated/cache --config-path .build/isolated/config --security-path .build/isolated/security --scratch-path .build/isolated/scratch --manifest-cache local --disable-dependency-cache --disable-automatic-resolution --explicit-target-dependency-import-check error

test-swift: swift-test

lint-check: fmt clippy swift-format swift-lint

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
  while IFS= read -r -d '' f; do
    total=$(wc -l < "$f")
    limit=200
    [[ "$f" == helper/vpt-macos/Tests/* ]] && limit=700
    if (( total > limit )); then
      printf 'FAIL %5d total (limit %d) %s\n' "$total" "$limit" "$f"
      status=1
    fi
  done < <(find helper/vpt-macos -path '*/.build' -prune -o -type f -name '*.swift' -print0)
  exit "$status"

ship: fmt clippy test doc gitleaks file-size swift-format swift-lint swift-build swift-test
