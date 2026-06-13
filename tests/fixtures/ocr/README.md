# OCR Fixtures

`trocr-hello.png` contains the printed text `HELLO` as black text on a white
background. It is used by the opt-in TrOCR ONNX external smoke test to verify
that native OCR execution decodes fixture intent, not only non-empty output.

The fixture was generated once with Pillow using a local system font and saved
as a small reviewed PNG. Keep OCR fixtures small and intentional; do not replace
them with generated model output.

Generation summary:

```bash
.external-test-tools/model-python-venv/bin/python - <<'PY'
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

out = Path("tests/fixtures/ocr/trocr-hello.png")
out.parent.mkdir(parents=True, exist_ok=True)
image = Image.new("RGB", (240, 80), "white")
draw = ImageDraw.Draw(image)
font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 42)
text = "HELLO"
bbox = draw.textbbox((0, 0), text, font=font)
draw.text(
    ((image.width - (bbox[2] - bbox[0])) // 2, (image.height - (bbox[3] - bbox[1])) // 2 - 2),
    text,
    fill="black",
    font=font,
)
image.save(out)
PY
```
