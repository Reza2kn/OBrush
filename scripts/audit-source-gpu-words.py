#!/usr/bin/env python3
"""Compare captured active WebGL VBO word hashes to actual public Save files.

Input readback JSON is a browser receipt, not an assertion oracle. A separate
normal-browser CDP observer must identify the active draw VBO, read it with
getBufferSubData, and hash its interleaved f32 position and normal words.
This script independently derives expected little-endian f32 words from each
normal Save/Undo/Redo project and rejects ambiguous or missing VBOs.
"""

import argparse
import gc
import hashlib
import json
import math
import struct
from pathlib import Path

from importlib.machinery import SourceFileLoader

HERE = Path(__file__).resolve().parent
verify = SourceFileLoader("source_worker_verify", str(HERE / "verify-source-worker-sustained.py")).load_module()
STAGES = ("before", "after", "stable", "undo", "redo")


def canonical_words(project_path, expected_vertices):
    with Path(project_path).open("rb") as source:
        document = json.load(source)
    mesh = document["mesh"]
    positions, normals = mesh["positions"], mesh["normals"]
    if len(positions) != expected_vertices or len(normals) != expected_vertices:
        raise ValueError("public mesh count disagrees with fixture")
    pos_hash, normal_hash = hashlib.sha256(), hashlib.sha256()
    for pos, normal in zip(positions, normals):
        if len(pos) != 3 or len(normal) != 3 or not all(math.isfinite(x) for x in pos + normal):
            raise ValueError("non-finite or incomplete position/normal")
        pos_hash.update(struct.pack("<fff", *pos))
        normal_hash.update(struct.pack("<fff", *normal))
    del document
    gc.collect()
    return pos_hash.hexdigest(), normal_hash.hexdigest()


def audit(result_path, browser_path):
    result, result_sha = verify.load_bounded(result_path)
    browser, _ = verify.load_bounded(browser_path, 1024 * 1024)
    expected_vertices = result["fixture"]["vertices"]
    report = {"result_sha256": result_sha, "browser_receipt_sha256":
              verify.sha_file(browser_path, 1024 * 1024), "stages": {}, "failed_checks": []}
    for stage in STAGES:
        public = result[stage]
        observed = browser.get("stages", {}).get(stage, {})
        save_sha = verify.sha_file(public["path"], 512 * 1024 * 1024)
        expected_pos, expected_norm = canonical_words(public["path"], expected_vertices)
        row = {
            "public_save_sha256": save_sha,
            "expected_position_words_sha256": expected_pos,
            "expected_normal_words_sha256": expected_norm,
            "actual_readback_sha256": observed.get("actual_readback_sha256"),
            "actual_position_words_sha256": observed.get("position_words_sha256"),
            "actual_normal_words_sha256": observed.get("normal_words_sha256"),
            "readback_bytes": observed.get("readback_bytes"),
            "active_vbo_candidates": observed.get("active_vbo_candidates"),
        }
        row["position_words_exact"] = (save_sha == public["sha256"]
                                       and expected_pos == row["actual_position_words_sha256"])
        row["normal_words_exact"] = (save_sha == public["sha256"]
                                     and expected_norm == row["actual_normal_words_sha256"])
        if not row["position_words_exact"] or not row["normal_words_exact"] or row["active_vbo_candidates"] != 1:
            report["failed_checks"].append(stage)
        report["stages"][stage] = row
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result", type=Path)
    parser.add_argument("browser_readback", type=Path)
    args = parser.parse_args()
    report = audit(args.result, args.browser_readback)
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if not report["failed_checks"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
