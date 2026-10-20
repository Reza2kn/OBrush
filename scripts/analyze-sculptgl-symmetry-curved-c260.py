#!/usr/bin/env python3
"""Posthoc C260 baseline-coordinate diagnostic; does not change its failed ID gate."""

import argparse
import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path


def analyze(path: Path):
    raw = path.read_bytes()
    case = json.loads(raw)
    source_before = list(zip(*[iter(case["reference_vertices_before"])] * 3))
    source_after = list(zip(*[iter(case["reference_vertices_after"])] * 3))
    own_before = [tuple(v) for v in case["obrush_vertices_before"]["positions"]]
    own_after = [tuple(v) for v in case["obrush_vertices_after"]["positions"]]
    assert len(source_before) == len(source_after) == len(own_before) == len(own_after) == 4514

    bins = defaultdict(list)
    for index, vector in enumerate(own_before):
        bins[tuple(round(value, 4) for value in vector)].append(index)
    remap = []
    used = set()
    for reference_id, vector in enumerate(source_before):
        candidates = [index for index in bins[tuple(round(value, 4) for value in vector)] if index not in used]
        if len(candidates) != 1:
            candidates = [index for index, own in enumerate(own_before) if index not in used and max(abs(a - b) for a, b in zip(vector, own)) < 1e-5]
        if len(candidates) != 1:
            raise ValueError(f"Reference vertex {reference_id} has {len(candidates)} baseline coordinate candidates")
        remap.append(candidates[0])
        used.add(candidates[0])
    assert len(used) == 4514
    max_baseline_error = max(abs(a - b) for index, own_id in enumerate(remap) for a, b in zip(source_before[index], own_before[own_id]))
    assert max_baseline_error < 1e-6

    source_delta = [tuple(a - b for a, b in zip(after, before)) for before, after in zip(source_before, source_after)]
    own_delta = [tuple(a - b for a, b in zip(own_after[own_id], own_before[own_id])) for own_id in remap]
    errors = [tuple(a - b for a, b in zip(own, source)) for own, source in zip(own_delta, source_delta)]
    source_changed = {index for index, vector in enumerate(source_delta) if max(map(abs, vector)) > 1e-6}
    own_changed = {index for index, vector in enumerate(own_delta) if max(map(abs, vector)) > 1e-6}
    per_axis = {}
    for axis, label in enumerate("xyz"):
        column = [vector[axis] for vector in errors]
        per_axis[label] = {"max_abs_error": max(map(abs, column)), "rms_error": math.sqrt(sum(value * value for value in column) / len(column))}
    worst = sorted(
        ({"reference_id": index, "obrush_id": remap[index], "max_abs_error": max(map(abs, vector)), "reference_delta": source_delta[index], "obrush_delta": own_delta[index]} for index, vector in enumerate(errors)),
        key=lambda row: row["max_abs_error"], reverse=True,
    )[:10]
    return {
        "classification": "posthoc exploratory baseline-coordinate diagnostic; preregistered C260 direct-ID gate remains failed",
        "receipt_sha256": hashlib.sha256(raw).hexdigest(),
        "reference_vertices": len(remap),
        "unique_baseline_matches": len(used),
        "unchanged_numeric_ids": sum(index == own_id for index, own_id in enumerate(remap)),
        "max_baseline_coordinate_error": max_baseline_error,
        "changed_reference_ids": len(source_changed),
        "changed_obrush_ids_after_mapping": len(own_changed),
        "changed_id_intersection": len(source_changed & own_changed),
        "changed_id_union": len(source_changed | own_changed),
        "changed_id_iou": len(source_changed & own_changed) / len(source_changed | own_changed),
        "max_abs_component_error": max(row["max_abs_error"] for row in worst),
        "rms_component_error": math.sqrt(sum(value * value for vector in errors for value in vector) / (3 * len(errors))),
        "per_axis": per_axis,
        "worst_ten": worst,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("receipts", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.receipts:
        report = analyze(path)
        dest = path.with_name("posthoc-baseline-coordinate-map.json")
        dest.write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({"source": str(path), "diagnostic": str(dest), "receipt_sha256": report["receipt_sha256"], "max_error": report["max_abs_component_error"], "iou": report["changed_id_iou"]}))


if __name__ == "__main__":
    main()
