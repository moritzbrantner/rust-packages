#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$ROOT/.external-test-tools/image-person-edit"
VIDEO_PATH="$WORK_DIR/me-at-the-zoo.webm"
FRAME_PATH="$WORK_DIR/person-frame.jpg"
VIDEO_URL="${ME_AT_THE_ZOO_VIDEO_URL:-https://upload.wikimedia.org/wikipedia/commons/e/e0/Me_at_the_zoo.webm}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 2
  fi
}

require_command ffmpeg
require_command python3

python3 - <<'PY'
import importlib.util
import sys

missing = [
    name
    for name in ("cv2", "numpy")
    if importlib.util.find_spec(name) is None
]
if missing:
    print(f"missing required Python modules: {', '.join(missing)}", file=sys.stderr)
    raise SystemExit(2)
PY

mkdir -p "$WORK_DIR"

if [[ -n "${ME_AT_THE_ZOO_VIDEO_PATH:-}" ]]; then
  cp "$ME_AT_THE_ZOO_VIDEO_PATH" "$VIDEO_PATH"
elif [[ ! -s "$VIDEO_PATH" ]]; then
VIDEO_URL="$VIDEO_URL" VIDEO_PATH="$VIDEO_PATH" python3 - <<'PY'
import os
import shutil
import urllib.request

request = urllib.request.Request(
    os.environ["VIDEO_URL"],
    headers={"User-Agent": "video-analysis-external-smoke/1.0"},
)
with urllib.request.urlopen(request) as response, open(os.environ["VIDEO_PATH"], "wb") as output:
    shutil.copyfileobj(response, output)
PY
fi

ffmpeg -y -v error -ss 00:00:08 -i "$VIDEO_PATH" -frames:v 1 -q:v 2 "$FRAME_PATH"
test -s "$FRAME_PATH"

ROOT="$ROOT" FRAME_PATH="$FRAME_PATH" python3 - <<'PY'
import base64
import json
import os
import subprocess
import sys

import cv2

root = os.environ["ROOT"]
frame_path = os.environ["FRAME_PATH"]
image = cv2.imread(frame_path, cv2.IMREAD_COLOR)
if image is None:
    print(f"failed to read extracted frame: {frame_path}", file=sys.stderr)
    raise SystemExit(2)
rgb = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
height, width, _ = rgb.shape
payload = {
    "task": "object_detection",
    "input": {
        "kind": "video_frame",
        "width": width,
        "height": height,
        "stride": width * 3,
        "pixel_format": "rgb24",
        "data_base64": base64.b64encode(rgb.tobytes()).decode("ascii"),
    },
}
completed = subprocess.run(
    [sys.executable, os.path.join(root, "scripts/opencv_person_detector.py")],
    input=json.dumps(payload),
    text=True,
    capture_output=True,
    check=False,
)
if completed.returncode != 0:
    print(completed.stderr, file=sys.stderr, end="")
    raise SystemExit(completed.returncode)
response = json.loads(completed.stdout)
predictions = [
    prediction
    for prediction in response.get("predictions", [])
    if prediction.get("label") == "person"
]
if not predictions:
    print("opencv person detector returned zero person predictions", file=sys.stderr)
    raise SystemExit(2)
PY

cat <<EOF
export IMAGE_PERSON_EDIT_INPUT="$FRAME_PATH"
export IMAGE_PERSON_EDIT_DETECTOR_COMMAND="python3"
export IMAGE_PERSON_EDIT_DETECTOR_ARGS="scripts/opencv_person_detector.py"
EOF
