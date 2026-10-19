#!/usr/bin/env python3
"""Fail-closed admission for SP-WORKER-01 browser performance receipts.

The older C203/C226/C233 records are accepted as historical ordinary-stroke
evidence, never as a complete SP-WORKER-01 pass: they lack GPU word readback.
No code path converts an opt-in worker microbenchmark into a held-stroke run.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROTOCOL = ROOT / "docs/SCULPTGL_WORKER_SUSTAINED_PROTOCOL.json"
MAX_RESULT = 128 * 1024 * 1024


def load_bounded(path, limit=MAX_RESULT):
    path = Path(path)
    if not path.is_file() or path.stat().st_size > limit:
        raise ValueError(f"missing or oversized receipt: {path}")
    raw = path.read_bytes()
    return json.loads(raw), hashlib.sha256(raw).hexdigest()


def sha_file(path, limit):
    path = Path(path)
    if not path.is_file() or not 0 < path.stat().st_size <= limit:
        raise ValueError(f"missing or oversized file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def percentile(values, portion):
    values = sorted(value for value in values if math.isfinite(value))
    if not values:
        raise ValueError("empty or non-finite samples")
    return values[math.ceil(portion * len(values)) - 1]


def check(report, key, predicate, detail=None):
    report["checks"][key] = {"pass": bool(predicate), "detail": detail}


def summarize(result_path, audit_path=None, gpu_path=None, worker_path=None, mode="ordinary"):
    protocol, protocol_sha = load_bounded(PROTOCOL, 64 * 1024)
    result, result_sha = load_bounded(result_path)
    report = {"status": "incomplete", "mode": mode, "result_sha256": result_sha,
              "protocol_sha256": protocol_sha, "checks": {}, "metrics": {},
              "scope": "One cohort only; full parity requires all gates and both scales."}
    cohort_label = result.get("fixture", {}).get("label")
    cohort = next((row for row in protocol["cohorts"] if row["label"] == cohort_label), None)
    check(report, "frozen_cohort", cohort is not None, cohort_label)
    if cohort is None:
        report["failed_checks"] = ["frozen_cohort"]
        return report
    fixture = result["fixture"]
    check(report, "fixture_sha256", fixture.get("sha256") == cohort["sgl_sha256"]
          and sha_file(fixture["path"], 128 * 1024 * 1024) == cohort["sgl_sha256"])
    check(report, "fixture_counts", (fixture.get("vertices"), fixture.get("triangles"))
          == (cohort["vertices"], cohort["triangles"]))
    check(report, "result_complete", str(result.get("status", "")).startswith("Completed actual sustained"))
    check(report, "browser_owned_closed", result.get("owned_closure_admission", {}).get("observed_owned_process_absent") is True
          and result.get("cleanup", {}).get("owned_servers_closed") == 1
          and not result.get("cleanup_errors"))
    check(report, "browser_errors_empty", not any(result.get(key) for key in
          ("exceptions", "console_errors", "failed_acknowledgments", "cleanup_errors")))
    check(report, "runtime_binding", bool(result.get("product_binding", {}).get("wasm_sha256"))
          and bool(result.get("browser", {}).get("product")))

    # Candidate receipts must prove the real UI pointer route. Historical
    # C203 receipts used that route but predate the explicit worker attestation.
    att = result.get("worker_sustained_attestation", {})
    pointer_ok = (att.get("input_path") == "ordinary_browser_held_pointer"
                  and att.get("manual_worker_api_calls") == 0
                  and att.get("wasm_dab_shortcuts") == 0
                  and att.get("ordinary_ui_file_import") is True)
    if mode == "worker":
        pointer_ok = pointer_ok and att.get("runtime_path") == "worker_geometry_published_to_normal_studio"
    else:
        pointer_ok = pointer_ok and att.get("runtime_path") == "ordinary_main_thread_source"
    check(report, "actual_pointer_path", pointer_ok,
          "Historical C203 files cannot certify the new worker-vs-ordinary path attestation")

    scheduled = result.get("scheduled", [])
    acknowledgments = len(scheduled) == cohort["requested_moves"] and all(
        isinstance(row.get("ack"), (int, float)) and not row.get("failure") for row in scheduled)
    check(report, "all_requested_moves_acknowledged", acknowledgments, len(scheduled))
    if acknowledgments:
        lateness = [row["sent"] - row["planned"] for row in scheduled]
        gates = protocol["gates"]
        report["metrics"].update({"pacing_p99_ms": percentile(lateness, .99),
                                  "pacing_max_ms": max(lateness)})
        check(report, "pacing_p99", report["metrics"]["pacing_p99_ms"] <= gates["pacing_p99_lateness_ms"])
        check(report, "pacing_max", report["metrics"]["pacing_max_ms"] <= gates["pacing_max_lateness_ms"])
    else:
        check(report, "pacing_p99", False)
        check(report, "pacing_max", False)
    release = result.get("release", {})
    duration = release.get("requested", 0) - scheduled[0].get("planned", 0) if scheduled else 0
    check(report, "full_duration", duration >= cohort["seconds"] * 1000 - 1000, duration)

    rec_meta = result.get("recorder", {})
    rec_ok = (rec_meta.get("bytes", 0) <= protocol["bounds"]["scalar_recorder_max_bytes"]
              and bool(rec_meta.get("path")) and bool(rec_meta.get("sha256")))
    if rec_ok:
        rec_ok = sha_file(rec_meta["path"], protocol["bounds"]["scalar_recorder_max_bytes"]) == rec_meta["sha256"]
    check(report, "recorder_hash_and_cap", rec_ok)
    if rec_ok:
        rec, _ = load_bounded(rec_meta["path"], protocol["bounds"]["scalar_recorder_max_bytes"])
        check(report, "recorder_no_errors", rec.get("error_count") == 0 and not rec.get("overflow"))
        raf = rec.get("appRAF", [])
        intervals = [b[2] - a[2] for a, b in zip(raf, raf[1:])]
        if intervals and all(math.isfinite(x) and x >= 0 for x in intervals):
            m = report["metrics"]
            m["active_raf_p95_ms"] = percentile(intervals, .95)
            m["active_raf_max_ms"] = max(intervals)
            m["app_frame_cpu_p95_ms"] = percentile([row[1] for row in raf], .95)
            if scheduled:
                in_stroke = [row for row in raf if scheduled[0]["planned"] <= row[2]
                             <= release.get("requested", 0)]
                stroke_intervals = [b[2] - a[2] for a, b in zip(in_stroke, in_stroke[1:])]
                if stroke_intervals:
                    m["stroke_raf_p95_ms"] = percentile(stroke_intervals, .95)
            m["release_ack_ms"] = release.get("acknowledged", math.inf) - release.get("requested", 0)
            gates = protocol["gates"]
            for name in ("active_raf_p95_ms", "active_raf_max_ms", "app_frame_cpu_p95_ms", "release_ack_ms"):
                check(report, name, m[name] <= gates[name], m[name])
        else:
            check(report, "active_raf_samples", False)
    else:
        check(report, "recorder_no_errors", False)
    tail = result.get("tail", [])
    end_queue = tail[-1].get("queue", {}) if tail else {}
    check(report, "tail_complete", len(tail) == protocol["bounds"]["tail_samples"])
    check(report, "queue_empty_after_tail", end_queue.get("pending_dabs") == 0)
    within_100 = [row for row in tail if 0 <= row.get("node", math.inf) - release.get("requested", 0)
                  <= protocol["gates"]["released_queue_zero_ms"]]
    check(report, "queue_empty_by_100ms", bool(within_100) and
          any(row.get("queue", {}).get("pending_dabs") == 0 for row in within_100),
          "A late first tail probe cannot prove the 100 ms gate")
    after_100 = [row.get("queue", {}).get("applied_dabs") for row in tail
                 if row.get("node", 0) - release.get("requested", 0) >= 100]
    check(report, "no_late_applied_dabs", bool(after_100) and len(set(after_100)) == 1)
    report["metrics"]["applied_dabs"] = end_queue.get("applied_dabs")
    check(report, "actual_dabs", isinstance(end_queue.get("applied_dabs"), int)
          and end_queue["applied_dabs"] > 0)
    check(report, "delivered_dom_trace", bool(att.get("trusted_dom_trace_sha256"))
          and att.get("delivered_dom_events", 0) > 0)

    files_ok = True
    file_hashes = {}
    for stage in ("before", "after", "stable", "undo", "redo"):
        item = result.get(stage, {})
        try:
            actual = sha_file(item["path"], protocol["bounds"]["public_save_max_bytes"])
            files_ok &= actual == item["sha256"]
            file_hashes[stage] = actual
        except (KeyError, ValueError):
            files_ok = False
    check(report, "public_files_hash_and_cap", files_ok)
    check(report, "public_history_exact", files_ok and
          file_hashes.get("before") == file_hashes.get("undo") and
          file_hashes.get("after") == file_hashes.get("stable") == file_hashes.get("redo"))
    if audit_path:
        audit, _ = load_bounded(audit_path, 1024 * 1024)
        shape_ok = (audit.get("result_sha256") == result_sha
                    and bool(audit.get("checks")) and all(audit["checks"].values())
                    and audit.get("shape", {}).get("maximum_changed_edge_stretch", math.inf)
                    <= protocol["gates"]["shape_max_edge_stretch_ratio"])
        check(report, "independent_whole_file_shape", shape_ok)
    else:
        check(report, "independent_whole_file_shape", False, "No independent audit supplied")

    if gpu_path:
        gpu, _ = load_bounded(gpu_path, 1024 * 1024)
        stages = gpu.get("stages", {})
        gpu_ok = gpu.get("result_sha256") == result_sha and all(
            stages.get(stage, {}).get("position_words_exact") is True
            and stages.get(stage, {}).get("normal_words_exact") is True
            and stages.get(stage, {}).get("active_vbo_candidates") == 1
            and stages.get(stage, {}).get("public_save_sha256") == file_hashes.get(stage)
            and bool(stages.get(stage, {}).get("actual_readback_sha256"))
            and stages.get(stage, {}).get("readback_bytes", math.inf)
            <= protocol["bounds"]["gpu_readback_max_bytes"]
            for stage in ("before", "after", "stable", "undo", "redo"))
        check(report, "gpu_position_normal_all_states", gpu_ok)
    else:
        check(report, "gpu_position_normal_all_states", False, "No actual VBO readback supplied")

    if mode == "worker":
        if worker_path:
            worker, _ = load_bounded(worker_path, 16 * 1024 * 1024)
            messages = worker.get("messages", [])
            transfer_ok = (worker.get("result_sha256") == result_sha and bool(messages)
                           and {"init", "dab", "publish"}.issubset({x.get("kind") for x in messages})
                           and all(0 < x.get("bytes", 0) <= protocol["bounds"]["worker_transfer_max_bytes_per_message"]
                                   and x.get("transfer_or_clone") in ("transfer", "clone") for x in messages)
                           and worker.get("accepted_dabs") == end_queue.get("applied_dabs")
                           and all(isinstance(worker.get(name), (int, float)) and worker[name] >= 0
                                   for name in ("main_serialize_cpu_ms", "main_validate_cpu_ms",
                                                "main_publish_cpu_ms", "worker_geometry_cpu_ms")))
            check(report, "worker_transport_and_stage_trace", transfer_ok)
        else:
            check(report, "worker_transport_and_stage_trace", False, "No transport receipt supplied")

    report["failed_checks"] = [key for key, value in report["checks"].items() if not value["pass"]]
    report["status"] = "qualified_single_cohort" if not report["failed_checks"] else "incomplete_or_failed"
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("result", type=Path)
    parser.add_argument("--audit", type=Path)
    parser.add_argument("--gpu-audit", type=Path)
    parser.add_argument("--worker-trace", type=Path)
    parser.add_argument("--mode", choices=("ordinary", "worker"), required=True)
    args = parser.parse_args()
    try:
        report = summarize(args.result, args.audit, args.gpu_audit, args.worker_trace, args.mode)
    except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
        report = {"status": "invalid_receipt", "mode": args.mode, "result": str(args.result),
                  "failure": f"{type(error).__name__}: {error}"}
    print(json.dumps(report, indent=2, allow_nan=False))
    return 0 if report["status"] == "qualified_single_cohort" else 1


if __name__ == "__main__":
    raise SystemExit(main())
