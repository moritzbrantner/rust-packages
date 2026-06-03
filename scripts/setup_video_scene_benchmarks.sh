#!/usr/bin/env bash
set -euo pipefail

DEFAULT_CORPUS_ROOT=".test-corpora/video-scene"
ROOT="${VIDEO_SCENE_CORPUS_ROOT:-$DEFAULT_CORPUS_ROOT}"
BBC_TARGET="${BBC_ROOT:-$ROOT/BBC}"
AUTOSHOT_TARGET="${AUTOSHOT_ROOT:-$ROOT/AutoShot}"
ARCHIVE_DIR="$ROOT/archives"
EXTERNAL_TOOLS_ROOT="${EXTERNAL_TEST_TOOLS_DIR:-.external-test-tools}"
GDOWN_VENV="$EXTERNAL_TOOLS_ROOT/video-scene-python-venv"

BBC_FIXED_URL="${BBC_FIXED_URL:-https://zenodo.org/records/14873790/files/fixed.zip}"
BBC_VIDEOS_URL="${BBC_VIDEOS_URL:-https://zenodo.org/records/14873790/files/videos.zip}"
AUTOSHOT_FILE_ID="${AUTOSHOT_FILE_ID:-17diRkLlNUUjHDooXdqFUTXYje2-x4Yt6}"
AUTOSHOT_DRIVE_URL="${AUTOSHOT_DRIVE_URL:-https://drive.google.com/file/d/$AUTOSHOT_FILE_ID/view}"
GDOWN_PIP_PACKAGE="${GDOWN_PIP_PACKAGE:-gdown==5.2.0}"

REFRESH="${REFRESH_VIDEO_SCENE_CORPORA:-0}"
DOWNLOAD_LARGE="${DOWNLOAD_LARGE_CORPORA:-0}"

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
usage: scripts/setup_video_scene_benchmarks.sh [target...]

Targets:
  all       Setup BBC and AutoShot. This is the default.
  bbc       Setup the BBC benchmark corpus only.
  autoshot  Setup the AutoShot benchmark corpus only.
  verify    Verify expected files exist without downloading.
  help      Print this help text.

Default paths:
  VIDEO_SCENE_CORPUS_ROOT=.test-corpora/video-scene
  BBC_ROOT=$VIDEO_SCENE_CORPUS_ROOT/BBC
  AUTOSHOT_ROOT=$VIDEO_SCENE_CORPUS_ROOT/AutoShot
  archives=$VIDEO_SCENE_CORPUS_ROOT/archives
  gdown venv=${EXTERNAL_TEST_TOOLS_DIR:-.external-test-tools}/video-scene-python-venv

Network downloads are opt-in. Set DOWNLOAD_LARGE_CORPORA=1 to download missing
archives, or provide local archives with BBC_FIXED_ARCHIVE, BBC_VIDEOS_ARCHIVE,
or AUTOSHOT_ARCHIVE.
EOF
}

ensure_download_allowed() {
  local label="$1"
  if [[ "$DOWNLOAD_LARGE" != "1" ]]; then
    die "$label is missing and live downloads are disabled. Set DOWNLOAD_LARGE_CORPORA=1 or provide a local archive override."
  fi
}

require_command() {
  local command_name="$1"
  command -v "$command_name" >/dev/null 2>&1 || die "$command_name is required"
}

count_files() {
  local dir="$1"
  local pattern="$2"
  if [[ ! -d "$dir" ]]; then
    echo 0
    return
  fi
  find "$dir" -maxdepth 1 -type f -name "$pattern" | wc -l | tr -d ' '
}

has_files() {
  local dir="$1"
  local pattern="$2"
  [[ -d "$dir" ]] && find "$dir" -maxdepth 1 -type f -name "$pattern" -print -quit | grep -q .
}

verify_bbc_fixed() {
  has_files "$BBC_TARGET/fixed" "*.txt"
}

verify_bbc_videos() {
  has_files "$BBC_TARGET/videos" "*.mp4"
}

verify_bbc() {
  local quiet="${1:-}"
  local videos=()
  local scenes=()
  local video_count=0
  local scene_count=0
  local pair_count=0

  if [[ -d "$BBC_TARGET/videos" ]]; then
    mapfile -t videos < <(find "$BBC_TARGET/videos" -maxdepth 1 -type f -name "*.mp4" | sort)
  fi
  if [[ -d "$BBC_TARGET/fixed" ]]; then
    mapfile -t scenes < <(find "$BBC_TARGET/fixed" -maxdepth 1 -type f -name "*.txt" | sort)
  fi

  video_count="${#videos[@]}"
  scene_count="${#scenes[@]}"
  if ((video_count == 0)); then
    [[ "$quiet" == "quiet" ]] || echo "BBC verification failed: no videos/*.mp4 files found in $BBC_TARGET" >&2
    return 1
  fi
  if ((scene_count == 0)); then
    [[ "$quiet" == "quiet" ]] || echo "BBC verification failed: no fixed/*.txt files found in $BBC_TARGET" >&2
    return 1
  fi
  if ((video_count != scene_count)); then
    [[ "$quiet" == "quiet" ]] || echo "BBC verification failed: $video_count videos but $scene_count annotations" >&2
    return 1
  fi

  local index
  for index in "${!videos[@]}"; do
    local video_base
    local scene_base
    local video_id
    local scene_id
    video_base="$(basename "${videos[$index]}" .mp4)"
    scene_base="$(basename "${scenes[$index]}" .txt)"
    video_id="${video_base#bbc_}"
    scene_id="${scene_base%%-*}"
    if [[ "$video_base" == "$video_id" || "$video_id" != "$scene_id" ]]; then
      [[ "$quiet" == "quiet" ]] || echo "BBC verification failed: $(basename "${videos[$index]}") does not pair with $(basename "${scenes[$index]}")" >&2
      return 1
    fi
  done

  [[ "$quiet" == "quiet" ]] || echo "BBC verified: $video_count videos paired with $scene_count annotations"
}

verify_autoshot() {
  local quiet="${1:-}"
  local videos=()
  local scenes=()
  local video_count=0
  local scene_count=0

  if [[ -d "$AUTOSHOT_TARGET/videos" ]]; then
    mapfile -t videos < <(find "$AUTOSHOT_TARGET/videos" -maxdepth 1 -type f -name "*.mp4" | sort)
  fi
  if [[ -d "$AUTOSHOT_TARGET/annotations" ]]; then
    mapfile -t scenes < <(find "$AUTOSHOT_TARGET/annotations" -maxdepth 1 -type f -name "*.txt" | sort)
  fi

  video_count="${#videos[@]}"
  scene_count="${#scenes[@]}"
  if ((video_count == 0)); then
    [[ "$quiet" == "quiet" ]] || echo "AutoShot verification failed: no videos/*.mp4 files found in $AUTOSHOT_TARGET" >&2
    return 1
  fi
  if ((scene_count == 0)); then
    [[ "$quiet" == "quiet" ]] || echo "AutoShot verification failed: no annotations/*.txt files found in $AUTOSHOT_TARGET" >&2
    return 1
  fi
  pair_count="$video_count"
  if ((scene_count < pair_count)); then
    pair_count="$scene_count"
  fi

  local index
  for ((index = 0; index < pair_count; index += 1)); do
    local video_id
    local scene_id
    video_id="$(basename "${videos[$index]}" .mp4)"
    scene_id="$(basename "${scenes[$index]}" .txt)"
    if [[ "$video_id" != "$scene_id" ]]; then
      [[ "$quiet" == "quiet" ]] || echo "AutoShot verification failed: $(basename "${videos[$index]}") does not pair with $(basename "${scenes[$index]}")" >&2
      return 1
    fi
  done

  [[ "$quiet" == "quiet" ]] || echo "AutoShot verified: $pair_count sorted pairs ($video_count videos, $scene_count annotations)"
}

download_archive() {
  local url="$1"
  local output="$2"
  local label="$3"

  if [[ -s "$output" && "$REFRESH" != "1" ]]; then
    echo "using existing $label archive: $output"
    return
  fi

  ensure_download_allowed "$label archive"
  mkdir -p "$(dirname "$output")"

  local tmp="${output}.tmp.$$"
  rm -f "$tmp"

  echo "downloading $label archive: $url"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --connect-timeout 30 --show-error -o "$tmp" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget --tries=3 --timeout=30 -O "$tmp" "$url"
  else
    rm -f "$tmp"
    die "curl or wget is required to download $label archive"
  fi

  [[ -s "$tmp" ]] || {
    rm -f "$tmp"
    die "downloaded $label archive is empty"
  }

  mv "$tmp" "$output"
  printf '%s\n' "$url" >"$output.source-url"
}

validate_local_archive() {
  local archive="$1"
  local label="$2"
  [[ -s "$archive" ]] || die "$label archive does not exist or is empty: $archive"
}

default_root_abs() {
  printf '%s/%s\n' "$(pwd -P)" "$DEFAULT_CORPUS_ROOT"
}

path_abs() {
  local path="$1"
  local parent
  local base
  parent="$(dirname "$path")"
  base="$(basename "$path")"
  if [[ -d "$parent" ]]; then
    printf '%s/%s\n' "$(cd "$parent" && pwd -P)" "$base"
  else
    printf '%s/%s\n' "$(pwd -P)" "$path"
  fi
}

clean_dir_for_extraction() {
  local dir="$1"
  if [[ ! -e "$dir" ]]; then
    mkdir -p "$dir"
    return
  fi
  if [[ -d "$dir" ]] && [[ -z "$(find "$dir" -mindepth 1 -maxdepth 1 -print -quit)" ]]; then
    return
  fi

  local abs_dir
  local allowed_root
  abs_dir="$(path_abs "$dir")"
  allowed_root="$(default_root_abs)"
  case "$abs_dir" in
    "$allowed_root"/*)
      rm -rf "$dir"
      mkdir -p "$dir"
      ;;
    *)
      die "refusing to remove existing data outside $DEFAULT_CORPUS_ROOT: $dir"
      ;;
  esac
}

copy_flat_files() {
  local source_dir="$1"
  local target_dir="$2"
  local pattern="$3"
  local label="$4"
  local copied=0

  [[ -d "$source_dir" ]] || die "extracted $label directory not found: $source_dir"
  mkdir -p "$target_dir"

  while IFS= read -r -d '' file; do
    cp "$file" "$target_dir/$(basename "$file")"
    copied=$((copied + 1))
  done < <(find "$source_dir" -maxdepth 1 -type f -name "$pattern" -print0)

  ((copied > 0)) || die "no $label files matching $pattern found in $source_dir"
}

extract_bbc_zip() {
  local archive="$1"
  local target_subdir="$2"
  local pattern="$3"
  local marker="$4"
  local label="$5"

  if [[ "$REFRESH" != "1" && -e "$BBC_TARGET/$marker" ]]; then
    if [[ "$target_subdir" == "fixed" ]] && verify_bbc_fixed; then
      echo "BBC $label already extracted"
      return
    fi
    if [[ "$target_subdir" == "videos" ]] && verify_bbc_videos; then
      echo "BBC $label already extracted"
      return
    fi
  fi
  if [[ "$REFRESH" != "1" && "$target_subdir" == "fixed" ]] && verify_bbc_fixed; then
    touch "$BBC_TARGET/$marker"
    echo "BBC $label already present"
    return
  fi
  if [[ "$REFRESH" != "1" && "$target_subdir" == "videos" ]] && verify_bbc_videos; then
    touch "$BBC_TARGET/$marker"
    echo "BBC $label already present"
    return
  fi

  require_command unzip
  unzip -tq "$archive" >/dev/null || die "BBC $label archive is not a valid zip: $archive"

  local tmp_dir="$ARCHIVE_DIR/extract-bbc-$target_subdir.tmp.$$"
  rm -rf "$tmp_dir"
  mkdir -p "$tmp_dir"

  unzip -q "$archive" -d "$tmp_dir"
  clean_dir_for_extraction "$BBC_TARGET/$target_subdir"

  while IFS= read -r -d '' file; do
    cp "$file" "$BBC_TARGET/$target_subdir/$(basename "$file")"
  done < <(find "$tmp_dir" -type f -name "$pattern" -print0)

  rm -rf "$tmp_dir"

  if [[ "$target_subdir" == "fixed" ]]; then
    verify_bbc_fixed || die "BBC fixed annotations did not extract into $BBC_TARGET/fixed"
  else
    verify_bbc_videos || die "BBC videos did not extract into $BBC_TARGET/videos"
  fi
  touch "$BBC_TARGET/$marker"
}

setup_bbc() {
  if [[ -n "${BBC_ROOT:-}" && -d "$BBC_ROOT" ]]; then
    verify_bbc || die "BBC_ROOT points to an invalid BBC layout: $BBC_ROOT"
    echo "using existing BBC_ROOT=$BBC_ROOT"
    return
  fi

  mkdir -p "$BBC_TARGET" "$ARCHIVE_DIR"
  if [[ "$REFRESH" != "1" && -e "$BBC_TARGET/.extracted-fixed" && -e "$BBC_TARGET/.extracted-videos" ]] && verify_bbc quiet; then
    echo "BBC already set up"
    return
  fi
  if [[ "$REFRESH" != "1" ]] && verify_bbc quiet; then
    touch "$BBC_TARGET/.extracted-fixed" "$BBC_TARGET/.extracted-videos"
    echo "BBC already present"
    return
  fi

  local fixed_archive="${BBC_FIXED_ARCHIVE:-$ARCHIVE_DIR/fixed.zip}"
  local videos_archive="${BBC_VIDEOS_ARCHIVE:-$ARCHIVE_DIR/videos.zip}"

  if [[ -n "${BBC_FIXED_ARCHIVE:-}" ]]; then
    validate_local_archive "$fixed_archive" "BBC fixed"
  else
    download_archive "$BBC_FIXED_URL" "$fixed_archive" "BBC fixed"
  fi

  if [[ -n "${BBC_VIDEOS_ARCHIVE:-}" ]]; then
    validate_local_archive "$videos_archive" "BBC videos"
  else
    download_archive "$BBC_VIDEOS_URL" "$videos_archive" "BBC videos"
  fi

  extract_bbc_zip "$fixed_archive" "fixed" "*.txt" ".extracted-fixed" "fixed annotations"
  extract_bbc_zip "$videos_archive" "videos" "*.mp4" ".extracted-videos" "videos"
  verify_bbc
}

ensure_gdown() {
  if command -v gdown >/dev/null 2>&1; then
    command -v gdown
    return
  fi

  ensure_download_allowed "AutoShot archive"
  require_command python3
  mkdir -p "$GDOWN_VENV"
  if [[ ! -x "$GDOWN_VENV/bin/python" ]]; then
    python3 -m venv "$GDOWN_VENV"
  fi
  if [[ ! -x "$GDOWN_VENV/bin/gdown" ]]; then
    "$GDOWN_VENV/bin/python" -m pip install "$GDOWN_PIP_PACKAGE" >&2
  fi
  [[ -x "$GDOWN_VENV/bin/gdown" ]] || die "gdown was not installed into $GDOWN_VENV"
  printf '%s\n' "$GDOWN_VENV/bin/gdown"
}

download_autoshot_archive() {
  local output="$ARCHIVE_DIR/AutoShot_test.tar.gz"

  if [[ -s "$output" && "$REFRESH" != "1" ]]; then
    echo "using existing AutoShot archive: $output" >&2
    printf '%s\n' "$output"
    return
  fi

  ensure_download_allowed "AutoShot archive"
  mkdir -p "$ARCHIVE_DIR"

  local gdown_bin
  local tmp="${output}.tmp.$$"
  gdown_bin="$(ensure_gdown)"
  rm -f "$tmp"

  echo "downloading AutoShot archive from Google Drive file ID $AUTOSHOT_FILE_ID" >&2
  if ! "$gdown_bin" --id "$AUTOSHOT_FILE_ID" -O "$tmp"; then
    rm -f "$tmp"
    cat >&2 <<EOF
error: gdown could not download AutoShot non-interactively.
Drive URL: $AUTOSHOT_DRIVE_URL
Fallback:
  AUTOSHOT_ARCHIVE=/path/to/AutoShot_test.tar.gz scripts/setup_video_scene_benchmarks.sh autoshot
EOF
    exit 1
  fi

  [[ -s "$tmp" ]] || {
    rm -f "$tmp"
    die "downloaded AutoShot archive is empty"
  }

  mv "$tmp" "$output"
  printf '%s\n' "$AUTOSHOT_DRIVE_URL" >"$output.source-url"
  printf '%s\n' "$output"
}

find_autoshot_root() {
  local extracted_root="$1"
  local candidate
  while IFS= read -r -d '' candidate; do
    if has_files "$candidate/videos" "*.mp4" && has_files "$candidate/annotations" "*.txt"; then
      printf '%s\n' "$candidate"
      return
    fi
  done < <(find "$extracted_root" -type d -print0)
  return 1
}

extract_autoshot_archive() {
  local archive="$1"

  if [[ "$REFRESH" != "1" && -e "$AUTOSHOT_TARGET/.extracted-autoshot" ]] && verify_autoshot quiet; then
    echo "AutoShot already extracted"
    return
  fi
  if [[ "$REFRESH" != "1" ]] && verify_autoshot quiet; then
    touch "$AUTOSHOT_TARGET/.extracted-autoshot"
    echo "AutoShot already present"
    return
  fi

  require_command tar
  if ! tar -tzf "$archive" >/dev/null; then
    cat >&2 <<EOF
error: AutoShot archive is not a valid tar.gz file: $archive
Drive URL: $AUTOSHOT_DRIVE_URL
Fallback:
  AUTOSHOT_ARCHIVE=/path/to/AutoShot_test.tar.gz scripts/setup_video_scene_benchmarks.sh autoshot
EOF
    exit 1
  fi

  local tmp_dir="$ARCHIVE_DIR/extract-autoshot.tmp.$$"
  local extracted_dataset_root
  rm -rf "$tmp_dir"
  mkdir -p "$tmp_dir"

  tar -xzf "$archive" -C "$tmp_dir"
  extracted_dataset_root="$(find_autoshot_root "$tmp_dir")" || {
    rm -rf "$tmp_dir"
    die "could not find AutoShot videos/ and annotations/ directories in $archive"
  }

  clean_dir_for_extraction "$AUTOSHOT_TARGET/videos"
  clean_dir_for_extraction "$AUTOSHOT_TARGET/annotations"
  copy_flat_files "$extracted_dataset_root/videos" "$AUTOSHOT_TARGET/videos" "*.mp4" "AutoShot video"
  copy_flat_files "$extracted_dataset_root/annotations" "$AUTOSHOT_TARGET/annotations" "*.txt" "AutoShot annotation"

  rm -rf "$tmp_dir"
  verify_autoshot || die "AutoShot extraction did not produce a valid layout"
  touch "$AUTOSHOT_TARGET/.extracted-autoshot"
}

setup_autoshot() {
  if [[ -n "${AUTOSHOT_ROOT:-}" && -d "$AUTOSHOT_ROOT" ]]; then
    verify_autoshot || die "AUTOSHOT_ROOT points to an invalid AutoShot layout: $AUTOSHOT_ROOT"
    echo "using existing AUTOSHOT_ROOT=$AUTOSHOT_ROOT"
    return
  fi

  mkdir -p "$AUTOSHOT_TARGET" "$ARCHIVE_DIR"
  if [[ "$REFRESH" != "1" && -e "$AUTOSHOT_TARGET/.extracted-autoshot" ]] && verify_autoshot quiet; then
    echo "AutoShot already set up"
    return
  fi
  if [[ "$REFRESH" != "1" ]] && verify_autoshot quiet; then
    touch "$AUTOSHOT_TARGET/.extracted-autoshot"
    echo "AutoShot already present"
    return
  fi

  local archive
  if [[ -n "${AUTOSHOT_ARCHIVE:-}" ]]; then
    archive="$AUTOSHOT_ARCHIVE"
    validate_local_archive "$archive" "AutoShot"
  else
    archive="$(download_autoshot_archive)"
  fi

  extract_autoshot_archive "$archive"
}

target_flags() {
  verify_bbc_target=0
  verify_autoshot_target=0
  local target
  for target in "$@"; do
    case "$target" in
      all)
        verify_bbc_target=1
        verify_autoshot_target=1
        ;;
      bbc)
        verify_bbc_target=1
        ;;
      autoshot)
        verify_autoshot_target=1
        ;;
    esac
  done
}

verify_targets() {
  local failed=0
  target_flags "$@"
  if ((verify_bbc_target == 1)); then
    verify_bbc || failed=1
  fi
  if ((verify_autoshot_target == 1)); then
    verify_autoshot || failed=1
  fi
  ((failed == 0)) || die "video scene benchmark verification failed"
}

print_verification_summary() {
  target_flags "$@"
  echo "verification summary:"
  if ((verify_bbc_target == 1)); then
    verify_bbc || true
  fi
  if ((verify_autoshot_target == 1)); then
    verify_autoshot || true
  fi
}

targets=()
verify_only=0

if (($# == 0)); then
  targets=("all")
else
  for arg in "$@"; do
    case "$arg" in
      help|--help|-h)
        usage
        exit 0
        ;;
      verify)
        verify_only=1
        ;;
      all|bbc|autoshot)
        targets+=("$arg")
        ;;
      *)
        usage >&2
        die "unknown target: $arg"
        ;;
    esac
  done
fi

if ((${#targets[@]} == 0)); then
  targets=("all")
fi

echo "video scene corpus root: $ROOT"

if ((verify_only == 1)); then
  verify_targets "${targets[@]}"
else
  for target in "${targets[@]}"; do
    case "$target" in
      all)
        setup_bbc
        setup_autoshot
        ;;
      bbc)
        setup_bbc
        ;;
      autoshot)
        setup_autoshot
        ;;
    esac
  done
fi

echo "BBC target: $BBC_TARGET"
echo "AutoShot target: $AUTOSHOT_TARGET"
if ((verify_only == 0)); then
  print_verification_summary "${targets[@]}"
fi
