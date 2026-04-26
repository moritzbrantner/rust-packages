#!/usr/bin/env python3

import json
import os
import sys
import urllib.error
import urllib.request


def main() -> int:
    request = json.load(sys.stdin)
    base_url = os.environ.get("COMFYUI_URL", "").rstrip("/")
    if not base_url:
        json.dump(
            {
                "status": "planned",
                "output_image": request.get("output_image"),
                "message": "set COMFYUI_URL to execute the workflow",
                "metadata": {},
            },
            sys.stdout,
        )
        return 0

    payload = json.dumps({"prompt": request["workflow"]}).encode("utf-8")
    http = urllib.request.Request(
        f"{base_url}/prompt",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(http) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as exc:
        raise SystemExit(f"comfyui request failed: {exc.read().decode('utf-8', 'ignore')}")
    except urllib.error.URLError as exc:
        raise SystemExit(f"comfyui request failed: {exc}")

    json.dump(
        {
            "status": "submitted",
            "output_image": request.get("output_image"),
            "message": "workflow submitted to ComfyUI",
            "metadata": {"prompt_id": str(body.get('prompt_id', ''))},
        },
        sys.stdout,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
