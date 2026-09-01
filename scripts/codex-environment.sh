#!/usr/bin/env bash
set -euo pipefail

mode="${1:-setup}"
if [[ "$mode" != "setup" && "$mode" != "maintenance" ]]; then
  printf 'usage: %s [setup|maintenance]\n' "$0" >&2
  exit 2
fi

root="$(git rev-parse --show-toplevel)"
config="$root/.repository-environment.toml"

if [[ ! -f "$config" ]]; then
  printf 'missing environment-v1 config: %s\n' "$config" >&2
  exit 2
fi

publish_path() {
  if [[ -n "${GITHUB_PATH:-}" ]]; then
    printf '%s\n' "$1" >> "$GITHUB_PATH"
  fi
}

desired_bun="$(python3 - "$root/package.json" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
if path.is_file():
    value = json.loads(path.read_text()).get('packageManager', '')
    if value.startswith('bun@'):
        print(value.split('@', 1)[1])
PY
)"
if [[ -n "$desired_bun" ]]; then
  if ! [[ "$desired_bun" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'Bun packageManager must use an exact version, got %s\n' "$desired_bun" >&2
    exit 2
  fi
  if ! command -v bun >/dev/null 2>&1 || [[ "$(bun --version)" != "$desired_bun" ]]; then
    curl -fsSL https://bun.sh/install | bash -s "bun-v${desired_bun}"
  fi
  export PATH="$HOME/.bun/bin:$PATH"
  publish_path "$HOME/.bun/bin"
fi

rust_toolchain="$(python3 - "$root/rust-toolchain.toml" <<'PY'
import pathlib, sys, tomllib
path = pathlib.Path(sys.argv[1])
if path.is_file():
    print(tomllib.loads(path.read_text()).get('toolchain', {}).get('channel', ''))
PY
)"
if [[ -n "$rust_toolchain" ]]; then
  if ! [[ "$rust_toolchain" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'Rust toolchain must use an exact version, got %s\n' "$rust_toolchain" >&2
    exit 2
  fi
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
  publish_path "$HOME/.cargo/bin"
  rustup toolchain install "$rust_toolchain" --profile minimal
  mapfile -t rust_components < <(python3 - "$root/rust-toolchain.toml" <<'PY'
import pathlib, sys, tomllib
path = pathlib.Path(sys.argv[1])
if path.is_file():
    for component in tomllib.loads(path.read_text()).get('toolchain', {}).get('components', []):
        print(component)
PY
  )
  for component in "${rust_components[@]}"; do
    rustup component add --toolchain "$rust_toolchain" "$component"
  done
  rustup default "$rust_toolchain"
fi

mapfile -t environment_commands < <(python3 - "$config" "$mode" <<'PY'
import sys, tomllib
with open(sys.argv[1], 'rb') as handle:
    data = tomllib.load(handle)
for command in data.get(sys.argv[2], {}).get('commands', []):
    print(command)
PY
)
for command in "${environment_commands[@]}"; do
  (cd "$root" && bash -lc "$command")
done

if [[ -n "$desired_bun" && "$(bun --version)" != "$desired_bun" ]]; then
  printf 'Bun preflight mismatch: expected %s, got %s\n' "$desired_bun" "$(bun --version)" >&2
  exit 1
fi
if [[ -n "$rust_toolchain" ]]; then
  observed_rust="$(cd "$root" && rustc --version | awk '{print $2}')"
  if [[ "$observed_rust" != "$rust_toolchain" ]]; then
    printf 'Rust preflight mismatch: expected %s, got %s\n' "$rust_toolchain" "$observed_rust" >&2
    exit 1
  fi
fi
if ! command -v wasm-pack >/dev/null 2>&1 || [[ "$(wasm-pack --version | awk '{print $2}')" != "0.14.0" ]]; then
  printf 'wasm-pack preflight mismatch: expected 0.14.0\n' >&2
  exit 1
fi
