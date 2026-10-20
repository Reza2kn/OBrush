#!/usr/bin/env python3
"""Audit C298 browser receipts, project bytes, and fixed GPU screenshot pixels.

Requires Pillow. Usage: python3 scripts/audit-project-assets-c298.py .artifacts/c298/<run>/result.json
"""
import base64
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageChops


def sha(data):
    return hashlib.sha256(data).hexdigest()


report_path = Path(sys.argv[1]).resolve()
run = report_path.parent
report = json.loads(report_path.read_text())
assert report["status"].startswith("passed"), report["status"]
assert report["wasm_sha256"] == report["served_wasm_sha256"]
assert report["fixture_sha256"] == {
    "mesh": "10ff58d736037223672442a8e4bbeb5697224b9c65cb09235472beddb8845819",
    "uv": "98956aa6d0717701f23d34c3414c371bafe1e6ed9c45914aec5217f454926b84",
    "matcap": "33cd9048b3665b9f24ba0739b9e2a2752400b3249a05cd4a31e3b7be3487ad07",
}
captures = {item["label"]: item for item in report["captures"]}
projects = {key: (run / f"{key}.obrush").read_bytes() for key in captures}
for key, item in captures.items():
    assert sha(projects[key]) == item["sha256"], key
    assert json.loads(projects[key])["version"] == 8, key
for key in ["uv-reopened", "uv-redo", "uv-after-invalid"]:
    assert projects[key] == projects["uv-original"], key
for key in ["matcap-undo", "matcap-fresh-open"]:
    assert projects[key] == projects["matcap-original"], key

uv = json.loads(projects["uv-original"])["assets"]
matcap = json.loads(projects["matcap-original"])["assets"]
assert (uv["shading"], uv["matcap_index"]) == (3, 3)
assert (matcap["shading"], matcap["matcap_index"]) == (4, 9)
assert uv["uv"] == matcap["uv"]
assert (uv["uv"]["width"], uv["uv"]["height"]) == (64, 64)
assert (matcap["matcaps"][0]["width"], matcap["matcaps"][0]["height"]) == (64, 64)
assert len(base64.b64decode(uv["uv"]["rgba_bottom_up"])) == 64 * 64 * 4
assert len(base64.b64decode(matcap["matcaps"][0]["rgba_bottom_up"])) == 64 * 64 * 4
assert "dimensions exceed" in report["invalid_notice"]

crop = (550, 240, 1080, 810)
points = [(650, 340), (980, 340), (650, 680), (980, 680)]
evidence = {
    "status": "passed exact public project and GPU screenshot crop checks",
    "browser": report["browser"],
    "wasm_sha256": report["wasm_sha256"],
    "fixture_sha256": report["fixture_sha256"],
    "project_sha256": {key: sha(data) for key, data in projects.items()},
    "canonical_rgba_sha256": {
        "uv": sha(base64.b64decode(uv["uv"]["rgba_bottom_up"])),
        "matcap": sha(base64.b64decode(matcap["matcaps"][0]["rgba_bottom_up"])),
    },
    "gpu_crop": {},
    "invalid_notice": report["invalid_notice"],
}
for original, matches in [
    ("uv-original", ["uv-reopened", "uv-after-invalid"]),
    ("matcap-original", ["matcap-undo", "matcap-fresh-open"]),
]:
    before = Image.open(run / f"{original}.png").convert("RGBA")
    before_crop = before.crop(crop)
    evidence["gpu_crop"][original] = {
        "sha256": sha(before_crop.tobytes()),
        "samples_rgba": [before.getpixel(point) for point in points],
        "exact_matches": {},
    }
    for name in matches:
        after = Image.open(run / f"{name}.png").convert("RGBA")
        equal = ImageChops.difference(before_crop, after.crop(crop)).getbbox() is None
        assert equal, f"GPU crop changed: {original} -> {name}"
        evidence["gpu_crop"][original]["exact_matches"][name] = equal

output = run / "audit.json"
output.write_text(json.dumps(evidence, indent=2) + "\n")
print(json.dumps({"status": evidence["status"], "audit": str(output), "sha256": sha(output.read_bytes())}))
