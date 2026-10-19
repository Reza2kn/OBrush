#!/usr/bin/env python3
"""Compare two captured active-GPU normal row streams, never project normals.

This is a diagnostic for ordinary After/Redo retained-history divergence and
for matched ordinary/Worker output. It does not claim reference parity.
"""

import argparse
import base64
import hashlib
import json
import struct
from pathlib import Path


def observed(receipt_path, stage):
    receipt = json.loads(Path(receipt_path).read_bytes())
    row = receipt["stages"][stage]
    if row["active_vbo_candidates"] != 1 or row["gl_error"] != 0:
        raise ValueError(f"{stage}: missing unique error-free active VBO")
    encoded = row.get("normal_words_base64")
    if not isinstance(encoded, str):
        raise ValueError(f"{stage}: raw normal words were not captured")
    words = base64.b64decode(encoded, validate=True)
    if len(words) % 12 or len(words) > 1_200_000:
        raise ValueError(f"{stage}: invalid bounded normal stream length")
    if hashlib.sha256(words).hexdigest() != row["normal_words_sha256"]:
        raise ValueError(f"{stage}: raw normal stream does not match readback hash")
    return row, words


def audit(left_project, left_receipt, left_stage,
          right_project, right_receipt, right_stage):
    left_save, right_save = Path(left_project).read_bytes(), Path(right_project).read_bytes()
    left, left_words = observed(left_receipt, left_stage)
    right, right_words = observed(right_receipt, right_stage)
    same_save = left_save == right_save
    same_position_words = left["position_words_sha256"] == right["position_words_sha256"]
    if len(left_words) != len(right_words):
        raise ValueError("captured vertex counts differ")
    changed = []
    for index in range(len(left_words) // 12):
        a, b = left_words[index * 12:(index + 1) * 12], right_words[index * 12:(index + 1) * 12]
        if a != b:
            if len(changed) < 16:
                changed.append({"index": index,
                                "left_bits": list(struct.unpack("<III", a)),
                                "right_bits": list(struct.unpack("<III", b))})
    return {"left_stage": left_stage, "right_stage": right_stage,
            "left_save_sha256": hashlib.sha256(left_save).hexdigest(),
            "right_save_sha256": hashlib.sha256(right_save).hexdigest(),
            "public_saves_exact": same_save,
            "gpu_positions_exact": same_position_words,
            "gpu_normal_words_exact": left_words == right_words,
            "differing_normal_rows": sum(
                left_words[i:i + 12] != right_words[i:i + 12]
                for i in range(0, len(left_words), 12)),
            "first_differing_rows": changed,
            "vertices": len(left_words) // 12,
            "interpretation": "comparison only; matched delivered inputs and renderer policy require separate evidence"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for side in ("left", "right"):
        parser.add_argument(f"{side}_project", type=Path)
        parser.add_argument(f"{side}_receipt", type=Path)
        parser.add_argument(f"{side}_stage")
    args = parser.parse_args()
    report = audit(args.left_project, args.left_receipt, args.left_stage,
                   args.right_project, args.right_receipt, args.right_stage)
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report["public_saves_exact"] and report["gpu_positions_exact"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
