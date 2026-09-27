#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit
isolation_root="$PWD/.build/isolated"
mkdir -p \
  "$isolation_root/home" \
  "$isolation_root/state" \
  "$isolation_root/config" \
  "$isolation_root/cache" \
  "$isolation_root/data" \
  "$isolation_root/tmp" \
  "$isolation_root/scratch" \
  "$isolation_root/security" \
  "$isolation_root/clang-cache"

developer_directory="${DEVELOPER_DIR:-}"
if [[ -z "$developer_directory" ]]; then
  developer_directory="$(xcode-select -p)"
fi

exec env -i \
  PATH="$PATH" \
  DEVELOPER_DIR="$developer_directory" \
  HOME="$isolation_root/home" \
  XDG_STATE_HOME="$isolation_root/state" \
  XDG_CONFIG_HOME="$isolation_root/config" \
  XDG_CACHE_HOME="$isolation_root/cache" \
  XDG_DATA_HOME="$isolation_root/data" \
  VPT_CONFIG="$isolation_root/config/vpt.toml" \
  TMPDIR="$isolation_root/tmp/" \
  CLANG_MODULE_CACHE_PATH="$isolation_root/clang-cache" \
  "$@"
