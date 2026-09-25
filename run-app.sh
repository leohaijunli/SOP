#!/usr/bin/env bash
# Start the field-sop desktop app.
#
#   ./run-app.sh [--repo /path/to/working-copy] [--release]
#
# The app is a Tauri shell: it loads the Svelte renderer built into ui/dist/. This script
# (re)builds the frontend if needed, builds the Rust binary, and starts the window. It
# leaves the app's settings and data in an isolated XDG directory unless --no-isolate.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# 1. Make sure cargo is on the path (mise/rustup both install it into ~/.cargo/bin).
if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

# 2. Arguments.
repo="${FIELD_SOP_REPO:-}"
profile="dev"
isolate=1
while [ $# -gt 0 ]; do
  case "$1" in
    --repo) repo="$2"; shift 2 ;;
    --release) profile="release"; shift ;;
    --no-isolate) isolate=0; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# 3. Build the Svelte renderer if it is not built yet (or rebuild on request).
if [ ! -d ui/node_modules ]; then
  echo "==> installing frontend dependencies"
  (cd ui && npm install)
fi
if [ ! -f ui/dist/index.html ] || [ "${REBUILD_UI:-0}" = "1" ]; then
  echo "==> building the renderer"
  (cd ui && npm run build)
fi

# 4. Pick the working copy: --repo wins, then the configured one (via the app), else here.
if [ -n "$repo" ]; then
  repo="$(cd "$repo" 2>/dev/null && pwd || echo "$repo")"
  printf '==> working copy: %s\n' "$repo"
fi

# 5. Build the Rust binary. Cargo names the default "dev" profile's output dir "debug".
echo "==> building the app ($profile)"
cargo build -p sop-app --profile "$profile"

outdir="$profile"
[ "$profile" = "dev" ] && outdir="debug"
bin="$here/target/${outdir}/field-sop"

# 6. Isolate config/data so the app never touches the machine's real settings, unless asked.
if [ "$isolate" = "1" ]; then
  cfg="${FIELD_SOP_CFG:-/tmp/field-sop-cfg}"
  data="${FIELD_SOP_DATA:-/tmp/field-sop-data}"
  mkdir -p "$cfg" "$data"
  export XDG_CONFIG_HOME="$cfg"
  export XDG_DATA_HOME="$data"
  echo "==> isolated XDG: config=$cfg data=$data"
fi

# 7. Launch. The app has no CLI args: it opens the configured working copy, else the
#    current directory. For --repo we cd there first so the app lands on that copy.
if [ -n "$repo" ]; then
  cd "$repo"
fi
exec "$bin"