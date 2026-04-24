#!/usr/bin/env python3
import base64
import json
import sys

import cv2
import numpy as np


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(2)


def decode_frame(payload: dict) -> np.ndarray:
    width = int(payload["width"])
    height = int(payload["height"])
    stride = int(payload["stride"])
    pixel_format = payload["pixel_format"]
    raw = base64.b64decode(payload["data_base64"])
    expected = height * stride
    if len(raw) != expected:
        fail(f"expected {expected} bytes, received {len(raw)}")

    frame = np.frombuffer(raw, dtype=np.uint8).reshape((height, stride))
    frame = frame[:, : width * 3].reshape((height, width, 3))
    if pixel_format == "rgb24":
        return cv2.cvtColor(frame, cv2.COLOR_RGB2BGR)
    if pixel_format == "bgr24":
        return frame
    fail(f"unsupported pixel format: {pixel_format}")


def detect_people(frame: np.ndarray) -> list[dict]:
    gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
    cascade = cv2.CascadeClassifier(
        cv2.data.haarcascades + "haarcascade_frontalface_default.xml"
    )
    faces = cascade.detectMultiScale(
        gray,
        scaleFactor=1.1,
        minNeighbors=5,
        minSize=(24, 24),
    )
    predictions = []
    for x, y, width, height in faces:
        predictions.append(
            {
                "kind": "object",
                "label": "person",
                "score": 0.99,
                "region": {
                    "x": float(x),
                    "y": float(y),
                    "width": float(width),
                    "height": float(height),
                    "normalized": False,
                },
                "attributes": {
                    "detector": "opencv_haar_face",
                },
            }
        )
    return predictions


def main() -> None:
    request = json.load(sys.stdin)
    if request.get("task") != "object_detection":
        fail(f"unsupported task: {request.get('task')!r}")

    payload = request.get("input", {})
    if payload.get("kind") != "video_frame":
        fail(f"unsupported input kind: {payload.get('kind')!r}")

    frame = decode_frame(payload)
    response = {"predictions": detect_people(frame)}
    json.dump(response, sys.stdout)


if __name__ == "__main__":
    main()
