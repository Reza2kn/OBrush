#!/usr/bin/env python3
"""Summarize the active recording and timed stroke in a sustained result.

The timed interval starts at the first scheduled move and ends at the release
request. The preregistered active-recorder gates include setup; stroke-only
values are separate diagnostics. This is not the shape/file/history audit.
"""

import hashlib
import json
import math
import sys
from pathlib import Path


def percentile95(values):
    if not values:
        raise ValueError("no timed frame samples")
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def summarize(path):
    source = Path(path)
    raw = source.read_bytes()
    result = json.loads(raw)
    if not result.get("status", "").startswith("Completed actual sustained"):
        raise ValueError(f"{path}: sustained run is incomplete")
    scheduled = result["scheduled"]
    if not scheduled:
        raise ValueError(f"{path}: no scheduled moves")
    start = scheduled[0]["planned"]
    end = result["release"]["requested"]
    if not (math.isfinite(start) and math.isfinite(end) and end > start):
        raise ValueError(f"{path}: invalid timed interval")
    record_path = Path(result["recorder"]["path"])
    record_bytes = record_path.read_bytes()
    if hashlib.sha256(record_bytes).hexdigest() != result["recorder"]["sha256"]:
        raise ValueError(f"{path}: recorder SHA-256 mismatch")
    recording = json.loads(record_bytes)
    if recording.get("error_count") or recording.get("overflow"):
        raise ValueError(f"{path}: recorder errors or overflow")
    frames = [row for row in recording["appRAF"] if start <= row[2] <= end]
    intervals = [b[2] - a[2] for a, b in zip(frames, frames[1:])]
    active_frames = recording["appRAF"]
    active_intervals = [b[2] - a[2] for a, b in zip(active_frames, active_frames[1:])]
    if any(value < 0 or not math.isfinite(value) for value in intervals):
        raise ValueError(f"{path}: invalid RAF interval")
    if any(value < 0 or not math.isfinite(value) for value in active_intervals):
        raise ValueError(f"{path}: invalid active RAF interval")
    release_ms = result["release"]["acknowledged"] - end
    output = {
        "side": result["side"],
        "result_path": str(source),
        "result_sha256": hashlib.sha256(raw).hexdigest(),
        "recorder_sha256": result["recorder"]["sha256"],
        "protocol_sha256": result["protocol_sha256"],
        "fixture_sha256": result["fixture"]["sha256"],
        "product_wasm_sha256": result["product_binding"]["wasm_sha256"] if result["side"] == "root" else None,
        "timed_start_ms": start,
        "timed_end_ms": end,
        "timed_frames": len(frames),
        "active_recorded_frames": len(active_frames),
        "active_raf_interval_p95_ms": percentile95(active_intervals),
        "active_raf_interval_max_ms": max(active_intervals),
        "active_app_frame_cpu_p95_ms": percentile95([row[1] for row in active_frames]),
        "stroke_raf_interval_p95_ms": percentile95(intervals),
        "stroke_raf_interval_max_ms": max(intervals),
        "stroke_app_frame_cpu_p95_ms": percentile95([row[1] for row in frames]),
        "release_acknowledgment_ms": release_ms,
        "preregistered_active_gates": {
            "raf_p95": percentile95(active_intervals) <= 33.34,
            "raf_max": max(active_intervals) <= 250,
            "app_cpu_p95": percentile95([row[1] for row in active_frames]) <= 16.667,
            "release_acknowledgment": release_ms <= 100,
        },
    }
    if result["side"] == "root":
        queue = result["tail"][-1]["queue"]
        paint = queue["paint_timing"]
        dab = queue["cpu_dab_timing"]
        output.update({
            "applied_dabs": queue["applied_dabs"],
            "coalesced_dabs": queue["coalesced_dabs"],
            "source_geometry_mean_ms": dab["source_geometry"]["mean_ms"],
            "source_retained_match_mean_ms": dab["source_geometry_retained_match"]["mean_ms"],
            "renderer_admission_mean_ms": paint["scene_admission"]["mean_ms"],
            "renderer_upload_mean_ms": paint["mesh_upload"]["mean_ms"],
            "renderer_callback_mean_ms": paint["paint"]["mean_ms"],
        })
    elif result["side"] == "source":
        source_dabs = recording["sourceDabs"]
        output.update({
            "source_callback_dabs": len(source_dabs),
            "source_callback_mean_ms": sum(row[1] for row in source_dabs) / len(source_dabs) if source_dabs else None,
            "source_callback_scope": "Pinned SculptGL recorded brush callback; not identical to OBrush's queued geometry timer",
        })
    else:
        raise ValueError(f"{path}: unknown benchmark side")
    return output


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit("usage: summarize-source-sustained.py RESULT.json [RESULT.json ...]")
    print(json.dumps([summarize(path) for path in sys.argv[1:]], indent=2))
